defmodule EthrexReplayWebWeb.HistoryLive do
  @moduledoc """
  LiveView for viewing job history.
  """
  use EthrexReplayWebWeb, :live_view

  alias EthrexReplayWeb.Jobs

  @impl true
  def mount(_params, _session, socket) do
    if connected?(socket) do
      Phoenix.PubSub.subscribe(EthrexReplayWeb.PubSub, "jobs")
    end

    jobs = Jobs.list_jobs(limit: 100)

    {:ok,
     socket
     |> assign(:page_title, "Job History")
     |> assign(:jobs, jobs)
     |> assign(:filter, "all")}
  end

  @impl true
  def render(assigns) do
    ~H"""
    <div class="min-h-screen flex flex-col">
      <!-- Navigation -->
      <nav class="navbar bg-base-200 border-b border-base-300 sticky top-0 z-50">
        <div class="container mx-auto px-4">
          <div class="flex-1">
            <a href="/" class="navbar-brand flex items-center gap-2">
              <svg class="w-8 h-8 text-primary" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 1.5l-9 5.25v10.5l9 5.25 9-5.25V6.75L12 1.5zm0 2.5l6.5 3.75L12 11.5 5.5 7.75 12 4zm-7 5.5l6 3.5v7l-6-3.5v-7zm14 0v7l-6 3.5v-7l6-3.5z" />
              </svg>
              <span>Ethrex Replay</span>
            </a>
          </div>
          <div class="flex-none">
            <ul class="menu menu-horizontal px-1 gap-2">
              <li><a href="/">Dashboard</a></li>
              <li><a href="/history" class="text-primary">History</a></li>
              <li><a href="/system">System</a></li>
            </ul>
          </div>
        </div>
      </nav>

      <main class="container mx-auto px-4 py-8 flex-1">
        <!-- Header -->
        <div class="flex flex-wrap items-center justify-between gap-4 mb-8">
          <div>
            <h1 class="text-3xl font-bold">Job History</h1>
            <p class="text-base-content/60 mt-1">View and manage past proof generation jobs</p>
          </div>
          <a href="/" class="btn btn-primary">
            <span class="hero-plus w-5 h-5"></span> New Job
          </a>
        </div>
        
    <!-- Filters -->
        <div class="flex flex-wrap gap-2 mb-6">
          <button
            phx-click="filter"
            phx-value-status="all"
            class={["btn btn-sm", if(@filter == "all", do: "btn-primary", else: "btn-ghost")]}
          >
            All
          </button>
          <button
            phx-click="filter"
            phx-value-status="running"
            class={["btn btn-sm", if(@filter == "running", do: "btn-primary", else: "btn-ghost")]}
          >
            <span class="status-dot status-dot-running"></span> Running
          </button>
          <button
            phx-click="filter"
            phx-value-status="completed"
            class={["btn btn-sm", if(@filter == "completed", do: "btn-primary", else: "btn-ghost")]}
          >
            <span class="status-dot status-dot-completed"></span> Completed
          </button>
          <button
            phx-click="filter"
            phx-value-status="failed"
            class={["btn btn-sm", if(@filter == "failed", do: "btn-primary", else: "btn-ghost")]}
          >
            <span class="status-dot status-dot-failed"></span> Failed
          </button>
          <button
            phx-click="filter"
            phx-value-status="pending"
            class={["btn btn-sm", if(@filter == "pending", do: "btn-primary", else: "btn-ghost")]}
          >
            <span class="status-dot status-dot-pending"></span> Pending
          </button>
          <button
            phx-click="filter"
            phx-value-status="cancelled"
            class={["btn btn-sm", if(@filter == "cancelled", do: "btn-primary", else: "btn-ghost")]}
          >
            <span class="status-dot status-dot-cancelled"></span> Cancelled
          </button>
        </div>
        
    <!-- Jobs Table -->
        <%= if filtered_jobs(@jobs, @filter) == [] do %>
          <div class="card bg-base-200 border border-base-300">
            <div class="card-body items-center text-center py-12">
              <span class="hero-inbox-stack w-16 h-16 text-base-content/30 mb-4"></span>
              <h3 class="text-xl font-medium">No jobs found</h3>
              <p class="text-base-content/60">
                <%= if @filter == "all" do %>
                  Start by creating a new proof generation job.
                <% else %>
                  No jobs with status "{@filter}".
                <% end %>
              </p>
              <a href="/" class="btn btn-primary mt-4">
                <span class="hero-plus w-5 h-5"></span> New Job
              </a>
            </div>
          </div>
        <% else %>
          <div class="overflow-x-auto">
            <table class="table table-zebra bg-base-200 rounded-lg">
              <thead>
                <tr class="text-base-content/60">
                  <th>Status</th>
                  <th>ZKVM</th>
                  <th>Action</th>
                  <th>Block</th>
                  <th>Network</th>
                  <th>Ethrex</th>
                  <th>Duration</th>
                  <th>Created</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                <%= for job <- filtered_jobs(@jobs, @filter) do %>
                  <tr
                    class="hover:bg-base-300/50 cursor-pointer"
                    phx-click="view_job"
                    phx-value-id={job.id}
                  >
                    <td>
                      <.status_badge status={job.status} />
                    </td>
                    <td class="font-medium">{String.upcase(job.zkvm)}</td>
                    <td>{String.capitalize(job.action)}</td>
                    <td class="font-mono text-sm">{job.block_number || "latest"}</td>
                    <td>{String.capitalize(job.network)}</td>
                    <td class="font-mono text-xs text-base-content/60">
                      {job.ethrex_branch || "main"}
                    </td>
                    <td class="text-base-content/60">
                      {format_duration(job.execution_time_ms)}
                    </td>
                    <td
                      class="text-base-content/60 text-sm"
                      phx-hook="LocalTime"
                      id={"job-created-#{job.id}"}
                      data-timestamp={NaiveDateTime.to_iso8601(job.inserted_at)}
                    >
                      {format_datetime(job.inserted_at)}
                    </td>
                    <td>
                      <a href={~p"/jobs/#{job.id}"} class="btn btn-ghost btn-sm">
                        <span class="hero-eye w-4 h-4"></span>
                      </a>
                    </td>
                  </tr>
                <% end %>
              </tbody>
            </table>
          </div>
        <% end %>
      </main>
      
    <!-- Footer -->
      <footer class="footer footer-center p-6 bg-base-200 text-base-content/60 mt-auto">
        <div>
          <p>
            Built with <span class="text-primary">ethrex</span>
            ·
            <a
              href="https://github.com/lambdaclass/ethrex-replay"
              class="link link-hover text-primary"
              target="_blank"
            >
              GitHub
            </a>
          </p>
        </div>
      </footer>
    </div>
    """
  end

  # Components

  attr :status, :string, required: true

  defp status_badge(assigns) do
    {bg_class, text_class, label} =
      case assigns.status do
        "pending" -> {"bg-neutral/30", "text-neutral-content", "Pending"}
        "queued" -> {"bg-neutral/30", "text-neutral-content", "Queued"}
        "running" -> {"bg-primary/20", "text-primary", "Running"}
        "completed" -> {"bg-success/20", "text-success", "Completed"}
        "failed" -> {"bg-error/20", "text-error", "Failed"}
        "cancelled" -> {"bg-warning/20", "text-warning", "Cancelled"}
        _ -> {"bg-neutral/30", "text-neutral-content", assigns.status}
      end

    assigns = assign(assigns, bg_class: bg_class, text_class: text_class, label: label)

    ~H"""
    <div class={["badge badge-sm gap-1.5", @bg_class, @text_class]}>
      <span class={"status-dot status-dot-#{@status}"}></span>
      {@label}
    </div>
    """
  end

  # Event Handlers

  @impl true
  def handle_event("filter", %{"status" => status}, socket) do
    {:noreply, assign(socket, :filter, status)}
  end

  @impl true
  def handle_event("view_job", %{"id" => id}, socket) do
    {:noreply, push_navigate(socket, to: ~p"/jobs/#{id}")}
  end

  # PubSub Handlers

  @impl true
  def handle_info({:job_created, _job}, socket) do
    jobs = Jobs.list_jobs(limit: 100)
    {:noreply, assign(socket, :jobs, jobs)}
  end

  @impl true
  def handle_info({:job_updated, _job}, socket) do
    jobs = Jobs.list_jobs(limit: 100)
    {:noreply, assign(socket, :jobs, jobs)}
  end

  @impl true
  def handle_info({:job_finished, _job_id}, socket) do
    jobs = Jobs.list_jobs(limit: 100)
    {:noreply, assign(socket, :jobs, jobs)}
  end

  @impl true
  def handle_info(_msg, socket) do
    {:noreply, socket}
  end

  # Helpers

  defp filtered_jobs(jobs, "all"), do: jobs

  defp filtered_jobs(jobs, status) do
    Enum.filter(jobs, &(&1.status == status))
  end

  defp format_duration(nil), do: "-"

  defp format_duration(ms) when is_integer(ms) do
    cond do
      ms < 1000 ->
        "#{ms}ms"

      ms < 60_000 ->
        "#{Float.round(ms / 1000, 1)}s"

      ms < 3_600_000 ->
        mins = div(ms, 60_000)
        secs = Float.round(rem(ms, 60_000) / 1000, 0) |> trunc()
        "#{mins}m #{secs}s"

      true ->
        hours = div(ms, 3_600_000)
        mins = div(rem(ms, 3_600_000), 60_000)
        "#{hours}h #{mins}m"
    end
  end

  defp format_datetime(datetime) do
    Calendar.strftime(datetime, "%b %d, %H:%M")
  end
end
