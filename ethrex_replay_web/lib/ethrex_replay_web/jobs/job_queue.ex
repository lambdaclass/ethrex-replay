defmodule EthrexReplayWeb.Jobs.JobQueue do
  @moduledoc """
  GenServer that manages the job queue, ensuring only one job runs at a time.
  """
  use GenServer

  require Logger

  alias EthrexReplayWeb.Jobs
  alias EthrexReplayWeb.Jobs.JobSupervisor

  defstruct [:current_job_id]

  # Client API

  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Submits a new job to the queue.
  Returns {:ok, job} if the job was created successfully.
  """
  def submit_job(attrs) do
    GenServer.call(__MODULE__, {:submit_job, attrs})
  end

  @doc """
  Cancels a job by ID.
  """
  def cancel_job(job_id) do
    GenServer.call(__MODULE__, {:cancel_job, job_id})
  end

  @doc """
  Returns the currently running job ID, if any.
  """
  def current_job do
    GenServer.call(__MODULE__, :current_job)
  end

  @doc """
  Returns the current queue status.
  """
  def status do
    GenServer.call(__MODULE__, :status)
  end

  # Server Callbacks

  @impl true
  def init(_opts) do
    # Subscribe to job events
    Phoenix.PubSub.subscribe(EthrexReplayWeb.PubSub, "jobs")

    # Check if there are any pending jobs to run
    send(self(), :check_queue)

    {:ok, %__MODULE__{current_job_id: nil}}
  end

  @impl true
  def handle_call({:submit_job, attrs}, _from, state) do
    # Create the job in the database
    case Jobs.create_job(attrs) do
      {:ok, job} ->
        Logger.info("Job #{job.id} created and queued")

        # Broadcast that a new job was created
        Phoenix.PubSub.broadcast(
          EthrexReplayWeb.PubSub,
          "jobs",
          {:job_created, job}
        )

        # Try to start the job if nothing is running
        send(self(), :check_queue)

        {:reply, {:ok, job}, state}

      {:error, changeset} ->
        {:reply, {:error, changeset}, state}
    end
  end

  @impl true
  def handle_call({:cancel_job, job_id}, _from, state) do
    cond do
      # If it's the currently running job, stop the server
      state.current_job_id == job_id ->
        EthrexReplayWeb.Jobs.JobServer.cancel(job_id)
        {:reply, :ok, state}

      # If it's a queued job, just mark it as cancelled
      job = Jobs.get_job(job_id) ->
        if job.status in ["pending", "queued"] do
          {:ok, _} = Jobs.mark_cancelled(job)
          {:reply, :ok, state}
        else
          {:reply, {:error, :invalid_status}, state}
        end

      true ->
        {:reply, {:error, :not_found}, state}
    end
  end

  @impl true
  def handle_call(:current_job, _from, state) do
    {:reply, state.current_job_id, state}
  end

  @impl true
  def handle_call(:status, _from, state) do
    counts = Jobs.count_by_status()

    status = %{
      current_job_id: state.current_job_id,
      pending: Map.get(counts, "pending", 0) + Map.get(counts, "queued", 0),
      running: Map.get(counts, "running", 0),
      completed: Map.get(counts, "completed", 0),
      failed: Map.get(counts, "failed", 0)
    }

    {:reply, status, state}
  end

  @impl true
  def handle_info(:check_queue, state) do
    new_state =
      if state.current_job_id == nil do
        # No job running, try to start the next one
        case Jobs.get_next_queued_job() do
          nil ->
            state

          job ->
            Logger.info("Starting next job from queue: #{job.id}")

            case JobSupervisor.start_job_server(job) do
              {:ok, _pid} ->
                %{state | current_job_id: job.id}

              {:error, reason} ->
                Logger.error("Failed to start job #{job.id}: #{inspect(reason)}")
                {:ok, _} = Jobs.mark_failed(job, "Failed to start: #{inspect(reason)}")
                # Try the next job
                send(self(), :check_queue)
                state
            end
        end
      else
        state
      end

    {:noreply, new_state}
  end

  @impl true
  def handle_info({:job_finished, job_id}, state) do
    new_state =
      if state.current_job_id == job_id do
        Logger.info("Job #{job_id} finished, checking queue for next job")
        # Schedule queue check
        send(self(), :check_queue)
        %{state | current_job_id: nil}
      else
        state
      end

    {:noreply, new_state}
  end

  @impl true
  def handle_info({:job_updated, _job}, state) do
    # Just ignore, this is for LiveView updates
    {:noreply, state}
  end

  @impl true
  def handle_info({:job_created, _job}, state) do
    # Job created, check if we should start it
    send(self(), :check_queue)
    {:noreply, state}
  end

  @impl true
  def handle_info(msg, state) do
    Logger.debug("JobQueue received unexpected message: #{inspect(msg)}")
    {:noreply, state}
  end
end
