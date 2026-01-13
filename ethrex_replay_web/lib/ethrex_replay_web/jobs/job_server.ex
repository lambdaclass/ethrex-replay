defmodule EthrexReplayWeb.Jobs.JobServer do
  @moduledoc """
  GenServer that manages execution of a single proof generation job.
  Uses Port to run the cargo command and stream output.
  """
  use GenServer, restart: :temporary

  require Logger

  alias EthrexReplayWeb.Jobs
  alias EthrexReplayWeb.Runner.CommandBuilder

  @timeout_ms 3_600_000  # 1 hour timeout

  defstruct [:job, :port, :buffer, :logs, :started_at]

  # Client API

  def start_link(opts) do
    job = Keyword.fetch!(opts, :job)
    GenServer.start_link(__MODULE__, job, name: via_tuple(job.id))
  end

  def get_logs(job_id) do
    GenServer.call(via_tuple(job_id), :get_logs)
  rescue
    _ -> []
  end

  def cancel(job_id) do
    GenServer.cast(via_tuple(job_id), :cancel)
  end

  defp via_tuple(job_id) do
    {:via, Registry, {EthrexReplayWeb.Jobs.JobRegistry, job_id}}
  end

  # Server Callbacks

  @impl true
  def init(job) do
    Process.flag(:trap_exit, true)

    # Schedule job start
    send(self(), :start_job)

    {:ok,
     %__MODULE__{
       job: job,
       port: nil,
       buffer: "",
       logs: [],
       started_at: nil
     }}
  end

  @impl true
  def handle_info(:start_job, state) do
    Logger.info("Starting job #{state.job.id}")

    # Update job status to running
    case Jobs.mark_running(state.job) do
      {:ok, updated_job} ->
        broadcast_status(updated_job, :running)

        # Build and execute command
        case CommandBuilder.build(updated_job) do
          {:ok, {executable, args}} ->
            Logger.info("Executing: #{executable} #{Enum.join(args, " ")}")

            # Save the command to the database
            command_string = "#{executable} #{Enum.join(args, " ")}"
            {:ok, updated_job} = Jobs.update_job(updated_job, %{command: command_string})

            # Get the project directory (parent of ethrex_replay_web)
            project_dir = get_project_dir()

            port =
              Port.open(
                {:spawn_executable, executable},
                [
                  :binary,
                  :exit_status,
                  :use_stdio,
                  :stderr_to_stdout,
                  args: args,
                  cd: project_dir,
                  env: build_env(updated_job)
                ]
              )

            # Set timeout
            Process.send_after(self(), :timeout, @timeout_ms)

            {:noreply,
             %{state | job: updated_job, port: port, started_at: System.monotonic_time(:millisecond)}}

          {:error, reason} ->
            Logger.error("Failed to build command: #{reason}")
            {:ok, failed_job} = Jobs.mark_failed(updated_job, reason)
            broadcast_status(failed_job, :failed)
            {:stop, :normal, %{state | job: failed_job}}
        end

      {:error, reason} ->
        Logger.error("Failed to mark job as running: #{inspect(reason)}")
        {:stop, :normal, state}
    end
  end

  @impl true
  def handle_info({port, {:data, data}}, %{port: port} = state) do
    # Process the incoming data
    {complete_lines, new_buffer} = process_buffer(state.buffer <> data)

    # Broadcast each log line
    Enum.each(complete_lines, fn line ->
      broadcast_log(state.job.id, line)
    end)

    new_logs = state.logs ++ complete_lines

    {:noreply, %{state | buffer: new_buffer, logs: new_logs}}
  end

  @impl true
  def handle_info({port, {:exit_status, exit_code}}, %{port: port} = state) do
    Logger.info("Job #{state.job.id} exited with code #{exit_code}")

    # Process any remaining buffer
    final_logs =
      if state.buffer != "" do
        state.logs ++ [state.buffer]
      else
        state.logs
      end

    # Calculate wall-clock time as fallback
    wall_clock_time =
      if state.started_at do
        System.monotonic_time(:millisecond) - state.started_at
      end

    # Parse results from logs
    results = parse_results(final_logs)

    # Prefer parsed execution time from logs, fall back to wall-clock time
    execution_time = results[:execution_time_ms] || wall_clock_time

    # Update job in database
    if exit_code == 0 do
      {:ok, completed_job} =
        Jobs.mark_completed(state.job, %{
          execution_time_ms: execution_time,
          proving_time_ms: results[:proving_time_ms],
          gas_used: results[:gas_used]
        })

      broadcast_status(completed_job, :completed)
      broadcast_job_finished(state.job.id)
      {:stop, :normal, %{state | job: completed_job, port: nil}}
    else
      error_msg = extract_error(final_logs) || "Process exited with code #{exit_code}"

      {:ok, failed_job} = Jobs.mark_failed(state.job, error_msg, exit_code)
      broadcast_status(failed_job, :failed)
      broadcast_job_finished(state.job.id)
      {:stop, :normal, %{state | job: failed_job, port: nil}}
    end
  end

  @impl true
  def handle_info(:timeout, state) do
    Logger.warning("Job #{state.job.id} timed out")

    if state.port do
      Port.close(state.port)
    end

    {:ok, failed_job} = Jobs.mark_failed(state.job, "Job timed out after #{@timeout_ms}ms")
    broadcast_status(failed_job, :failed)
    broadcast_job_finished(state.job.id)
    {:stop, :normal, %{state | job: failed_job, port: nil}}
  end

  @impl true
  def handle_info({:EXIT, port, reason}, %{port: port} = state) do
    Logger.warning("Job #{state.job.id} port exited unexpectedly: #{inspect(reason)}")

    {:ok, failed_job} = Jobs.mark_failed(state.job, "Process terminated unexpectedly: #{inspect(reason)}")
    broadcast_status(failed_job, :failed)
    broadcast_job_finished(state.job.id)
    {:stop, :normal, %{state | job: failed_job, port: nil}}
  end

  @impl true
  def handle_info(msg, state) do
    Logger.debug("JobServer received unexpected message: #{inspect(msg)}")
    {:noreply, state}
  end

  @impl true
  def handle_call(:get_logs, _from, state) do
    {:reply, state.logs, state}
  end

  @impl true
  def handle_cast(:cancel, state) do
    Logger.info("Cancelling job #{state.job.id}")

    if state.port do
      Port.close(state.port)
    end

    {:ok, cancelled_job} = Jobs.mark_cancelled(state.job)
    broadcast_status(cancelled_job, :cancelled)
    broadcast_job_finished(state.job.id)
    {:stop, :normal, %{state | job: cancelled_job, port: nil}}
  end

  @impl true
  def terminate(reason, state) do
    Logger.info("JobServer #{state.job.id} terminating: #{inspect(reason)}")

    if state.port do
      Port.close(state.port)
    end

    :ok
  end

  # Private functions

  defp get_project_dir do
    # Get the project directory from config or use the parent directory
    Application.get_env(:ethrex_replay_web, :project_dir) ||
      Path.expand("../../../..", __DIR__)
  end

  defp build_env(job) do
    base_env = [
      {~c"RUST_LOG", ~c"info"},
      {~c"RUST_BACKTRACE", ~c"1"}
    ]

    # Add GPU-specific env vars
    gpu_env =
      if job.resource == "gpu" do
        case job.zkvm do
          "sp1" -> [{~c"SP1_PROVER", ~c"cuda"}]
          _ -> [{~c"CUDA_VISIBLE_DEVICES", ~c"0"}]
        end
      else
        []
      end

    base_env ++ gpu_env
  end

  defp process_buffer(data) do
    lines = String.split(data, ~r/\r?\n/)

    case lines do
      [] ->
        {[], ""}

      [single] ->
        {[], single}

      multiple ->
        {complete, [incomplete]} = Enum.split(multiple, -1)
        {Enum.reject(complete, &(&1 == "")), incomplete}
    end
  end

  defp parse_results(logs) do
    %{
      execution_time_ms: extract_execution_time(logs),
      proving_time_ms: extract_proving_time(logs),
      gas_used: extract_gas_used(logs)
    }
  end

  defp extract_execution_time(logs) do
    # Look for patterns like "Execution Time: 07s 890ms" or "Execution Time: 1m 23s 456ms"
    Enum.find_value(logs, fn line ->
      cond do
        # Pattern: "Execution Time: XXs YYYms"
        match = Regex.run(~r/[Ee]xecution\s+[Tt]ime[:\s]+(\d+)s\s+(\d+)ms/i, line) ->
          [_, seconds, ms] = match
          String.to_integer(seconds) * 1000 + String.to_integer(ms)

        # Pattern: "Execution Time: X.XXs"
        match = Regex.run(~r/[Ee]xecution\s+[Tt]ime[:\s]+(\d+\.?\d*)\s*s(?!\s*\d)/i, line) ->
          [_, seconds] = match
          round(parse_float(seconds) * 1000)

        # Pattern: "Execution Time: XXm YYs ZZZms"
        match = Regex.run(~r/[Ee]xecution\s+[Tt]ime[:\s]+(\d+)m\s+(\d+)s\s+(\d+)ms/i, line) ->
          [_, minutes, seconds, ms] = match
          String.to_integer(minutes) * 60 * 1000 +
            String.to_integer(seconds) * 1000 +
            String.to_integer(ms)

        true ->
          nil
      end
    end)
  end

  defp extract_proving_time(logs) do
    # Look for patterns like "Proving time: 123.45s" or "Proof generated in 123.45s"
    Enum.find_value(logs, fn line ->
      cond do
        match = Regex.run(~r/[Pp]roving\s+time[:\s]+(\d+\.?\d*)\s*s/i, line) ->
          [_, seconds] = match
          round(parse_float(seconds) * 1000)

        match = Regex.run(~r/[Pp]roof\s+generated\s+in\s+(\d+\.?\d*)\s*s/i, line) ->
          [_, seconds] = match
          round(parse_float(seconds) * 1000)

        match = Regex.run(~r/(\d+)m\s+(\d+\.?\d*)s/, line) ->
          [_, minutes, seconds] = match
          mins = String.to_integer(minutes)
          secs = parse_float(seconds)
          round((mins * 60 + secs) * 1000)

        true ->
          nil
      end
    end)
  end

  defp parse_float(str) do
    case Float.parse(str) do
      {f, _} -> f
      :error -> String.to_integer(str) * 1.0
    end
  end

  defp extract_gas_used(logs) do
    Enum.find_value(logs, fn line ->
      case Regex.run(~r/[Gg]as\s+used[:\s]+(\d+)/i, line) do
        [_, gas] -> String.to_integer(gas)
        _ -> nil
      end
    end)
  end

  defp extract_error(logs) do
    # Look for error patterns in the last few lines
    logs
    |> Enum.reverse()
    |> Enum.take(10)
    |> Enum.find(fn line ->
      String.contains?(String.downcase(line), "error") or
        String.contains?(String.downcase(line), "failed") or
        String.contains?(String.downcase(line), "panic")
    end)
  end

  defp broadcast_log(job_id, line) do
    Phoenix.PubSub.broadcast(
      EthrexReplayWeb.PubSub,
      "job:#{job_id}",
      {:job_log, job_id, line}
    )
  end

  defp broadcast_status(job, status) do
    Phoenix.PubSub.broadcast(
      EthrexReplayWeb.PubSub,
      "job:#{job.id}",
      {:job_status, job.id, status}
    )

    Phoenix.PubSub.broadcast(
      EthrexReplayWeb.PubSub,
      "jobs",
      {:job_updated, job}
    )
  end

  defp broadcast_job_finished(job_id) do
    Phoenix.PubSub.broadcast(
      EthrexReplayWeb.PubSub,
      "jobs",
      {:job_finished, job_id}
    )
  end
end
