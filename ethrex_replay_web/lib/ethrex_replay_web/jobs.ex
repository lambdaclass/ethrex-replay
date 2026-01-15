defmodule EthrexReplayWeb.Jobs do
  @moduledoc """
  Context for managing proof generation jobs.
  """
  import Ecto.Query
  alias EthrexReplayWeb.{Repo, Job}

  @doc """
  Creates a new job with the given attributes.
  """
  def create_job(attrs \\ %{}) do
    %Job{}
    |> Job.changeset(attrs)
    |> Repo.insert()
  end

  @doc """
  Updates an existing job.
  """
  def update_job(%Job{} = job, attrs) do
    job
    |> Job.changeset(attrs)
    |> Repo.update()
  end

  @doc """
  Gets a single job by ID.
  """
  def get_job(id) do
    Repo.get(Job, id)
  end

  @doc """
  Gets a single job by ID, raises if not found.
  """
  def get_job!(id) do
    Repo.get!(Job, id)
  end

  @doc """
  Lists all jobs, ordered by most recent first.
  """
  def list_jobs(opts \\ []) do
    limit = Keyword.get(opts, :limit, 50)

    Job
    |> order_by([j], desc: j.inserted_at)
    |> limit(^limit)
    |> Repo.all()
  end

  @doc """
  Lists jobs with a specific status.
  """
  def list_jobs_by_status(status) do
    Job
    |> where([j], j.status == ^status)
    |> order_by([j], asc: j.inserted_at)
    |> Repo.all()
  end

  @doc """
  Gets the currently running job, if any.
  """
  def get_running_job do
    Job
    |> where([j], j.status == "running")
    |> limit(1)
    |> Repo.one()
  end

  @doc """
  Gets the next pending/queued job to run.
  """
  def get_next_queued_job do
    Job
    |> where([j], j.status in ["pending", "queued"])
    |> order_by([j], asc: j.inserted_at)
    |> limit(1)
    |> Repo.one()
  end

  @doc """
  Counts jobs by status.
  """
  def count_by_status do
    Job
    |> group_by([j], j.status)
    |> select([j], {j.status, count(j.id)})
    |> Repo.all()
    |> Map.new()
  end

  @doc """
  Marks a job as running.
  """
  def mark_running(%Job{} = job) do
    update_job(job, %{status: "running"})
  end

  @doc """
  Marks a job as completed with results.
  """
  def mark_completed(%Job{} = job, attrs \\ %{}) do
    attrs = Map.merge(attrs, %{status: "completed"})
    update_job(job, attrs)
  end

  @doc """
  Marks a job as failed with error info.
  """
  def mark_failed(%Job{} = job, error, exit_code \\ nil) do
    update_job(job, %{
      status: "failed",
      error: error,
      exit_code: exit_code
    })
  end

  @doc """
  Marks a job as cancelled.
  """
  def mark_cancelled(%Job{} = job) do
    update_job(job, %{status: "cancelled"})
  end

  @doc """
  Deletes a job.
  """
  def delete_job(%Job{} = job) do
    Repo.delete(job)
  end

  @doc """
  Returns the changeset for a job (useful for forms).
  """
  def change_job(%Job{} = job, attrs \\ %{}) do
    Job.changeset(job, attrs)
  end
end
