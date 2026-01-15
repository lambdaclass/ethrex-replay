defmodule EthrexReplayWeb.Jobs.JobSupervisor do
  @moduledoc """
  DynamicSupervisor for managing JobServer processes.
  """
  use DynamicSupervisor

  alias EthrexReplayWeb.Jobs.JobServer

  def start_link(opts) do
    DynamicSupervisor.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @impl true
  def init(_opts) do
    DynamicSupervisor.init(strategy: :one_for_one)
  end

  @doc """
  Starts a new JobServer for the given job.
  """
  def start_job_server(job) do
    child_spec = {JobServer, job: job}
    DynamicSupervisor.start_child(__MODULE__, child_spec)
  end

  @doc """
  Stops a JobServer by its job ID.
  """
  def stop_job_server(job_id) do
    case Registry.lookup(EthrexReplayWeb.Jobs.JobRegistry, job_id) do
      [{pid, _}] ->
        DynamicSupervisor.terminate_child(__MODULE__, pid)

      [] ->
        {:error, :not_found}
    end
  end

  @doc """
  Returns all running job servers.
  """
  def list_running_jobs do
    DynamicSupervisor.which_children(__MODULE__)
    |> Enum.map(fn {_, pid, _, _} ->
      case Registry.keys(EthrexReplayWeb.Jobs.JobRegistry, pid) do
        [job_id] -> job_id
        _ -> nil
      end
    end)
    |> Enum.reject(&is_nil/1)
  end
end
