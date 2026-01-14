defmodule EthrexReplayWebWeb.JobLive do
  @moduledoc """
  LiveView for viewing individual job details and real-time logs.
  """
  use EthrexReplayWebWeb, :live_view

  alias EthrexReplayWeb.Jobs
  alias EthrexReplayWeb.Jobs.{JobServer, JobQueue}

  @impl true
  def mount(%{"id" => id}, _session, socket) do
    case Jobs.get_job(id) do
      nil ->
        {:ok,
         socket
         |> put_flash(:error, "Job not found")
         |> push_navigate(to: ~p"/")}

      job ->
        if connected?(socket) do
          Phoenix.PubSub.subscribe(EthrexReplayWeb.PubSub, "job:#{id}")
        end

        # Get existing logs if job is running
        logs =
          if job.status == "running" do
            try do
              JobServer.get_logs(id)
            rescue
              _ -> []
            end
          else
            []
          end

        {:ok,
         socket
         |> assign(:page_title, "Job #{short_id(id)}")
         |> assign(:job, job)
         |> assign(:logs, logs)
         |> assign(:auto_scroll, true)}
    end
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
              <li><a href="/history">History</a></li>
              <li><a href="/system">System</a></li>
            </ul>
          </div>
        </div>
      </nav>

      <main class="container mx-auto px-4 py-8 flex-1">
        <!-- Breadcrumb -->
        <div class="breadcrumbs text-sm mb-6">
          <ul>
            <li><a href="/">Dashboard</a></li>
            <li><a href="/history">Jobs</a></li>
            <li class="text-primary">Job {short_id(@job.id)}</li>
          </ul>
        </div>

        <div class="grid lg:grid-cols-3 gap-8">
          <!-- Main Content -->
          <div class="lg:col-span-2 space-y-6">
            <!-- Status Header -->
            <div class="card bg-base-200 border border-base-300">
              <div class="card-body">
                <div class="flex flex-wrap items-center justify-between gap-4">
                  <div class="flex items-center gap-4">
                    <.status_badge status={@job.status} />
                    <div>
                      <h1 class="text-2xl font-bold">
                        {String.upcase(@job.zkvm)} · {String.capitalize(@job.action)}
                      </h1>
                      <p class="text-base-content/60 text-sm">
                        Block {@job.block_number || "latest"} on {String.capitalize(@job.network)}
                      </p>
                    </div>
                  </div>

                  <%= if @job.status == "running" do %>
                    <button
                      phx-click="cancel"
                      class="btn btn-outline btn-error btn-sm"
                      data-confirm="Are you sure you want to cancel this job?"
                    >
                      <span class="hero-stop w-4 h-4"></span> Cancel
                    </button>
                  <% end %>
                </div>
              </div>
            </div>
            
    <!-- Command -->
            <%= if @job.command do %>
              <div class="card bg-base-200 border border-base-300">
                <div class="card-body">
                  <h2 class="card-title text-lg mb-2">
                    <span class="hero-command-line w-5 h-5"></span> Command
                  </h2>
                  <div class="command-preview text-sm">
                    {@job.command}
                  </div>
                </div>
              </div>
            <% end %>
            
    <!-- Logs -->
            <div class="card bg-base-200 border border-base-300">
              <div class="card-body">
                <div class="flex items-center justify-between mb-2">
                  <h2 class="card-title text-lg">
                    <span class="hero-document-text w-5 h-5"></span> Logs
                  </h2>
                  <label class="label cursor-pointer gap-2">
                    <span class="label-text text-sm">Auto-scroll</span>
                    <input
                      type="checkbox"
                      class="toggle toggle-primary toggle-sm"
                      checked={@auto_scroll}
                      phx-click="toggle_auto_scroll"
                    />
                  </label>
                </div>

                <div
                  id="log-viewer"
                  phx-hook="LogStream"
                  data-auto-scroll={to_string(@auto_scroll)}
                  class="log-viewer bg-base-300/50 rounded-lg p-4 h-96 overflow-y-auto"
                >
                  <%= if @logs == [] do %>
                    <%= if @job.status == "running" do %>
                      <div class="flex items-center gap-2 text-base-content/50">
                        <span class="loading loading-dots loading-sm"></span> Waiting for output...
                      </div>
                    <% else %>
                      <div class="text-base-content/50">No logs available</div>
                    <% end %>
                  <% else %>
                    <%= for {line, idx} <- Enum.with_index(@logs) do %>
                      <div class={"py-0.5 #{log_line_class(line)}"}>
                        <span class="text-base-content/30 select-none mr-4 inline-block w-8 text-right">
                          {idx + 1}
                        </span>
                        <span>{line}</span>
                      </div>
                    <% end %>
                  <% end %>
                </div>
              </div>
            </div>
            
    <!-- Results (if completed) -->
            <%= if @job.status == "completed" do %>
              <div class="card bg-base-200 border border-success/30">
                <div class="card-body">
                  <h2 class="card-title text-lg text-success">
                    <span class="hero-check-badge w-5 h-5"></span> Results
                  </h2>
                  <div class="grid grid-cols-2 md:grid-cols-3 gap-4 mt-4">
                    <%= if @job.execution_time_ms do %>
                      <.result_metric
                        label="Execution Time"
                        value={format_duration(@job.execution_time_ms)}
                      />
                    <% end %>
                    <%= if @job.proving_time_ms do %>
                      <.result_metric
                        label="Proving Time"
                        value={format_duration(@job.proving_time_ms)}
                      />
                    <% end %>
                    <%= if @job.gas_used do %>
                      <.result_metric
                        label="Gas Used"
                        value={format_number(@job.gas_used)}
                      />
                    <% end %>
                  </div>
                </div>
              </div>
            <% end %>
            
    <!-- Error (if failed) -->
            <%= if @job.status == "failed" && @job.error do %>
              <div class="card bg-base-200 border border-error/30">
                <div class="card-body">
                  <h2 class="card-title text-lg text-error">
                    <span class="hero-exclamation-triangle w-5 h-5"></span> Error
                  </h2>
                  <div class="bg-error/10 text-error rounded-lg p-4 mt-2 font-mono text-sm">
                    {@job.error}
                  </div>
                  <%= if @job.exit_code do %>
                    <div class="text-sm text-base-content/50 mt-2">
                      Exit code: {@job.exit_code}
                    </div>
                  <% end %>
                </div>
              </div>
            <% end %>
          </div>
          
    <!-- Sidebar -->
          <div class="space-y-6">
            <!-- Job Details -->
            <div class="card bg-base-200 border border-base-300">
              <div class="card-body">
                <h2 class="card-title text-lg">
                  <span class="hero-information-circle w-5 h-5"></span> Details
                </h2>
                <div class="space-y-3 mt-2">
                  <.detail_row label="Job ID" value={short_id(@job.id)} />
                  <.detail_row label="ZKVM" value={String.upcase(@job.zkvm)} />
                  <.detail_row label="Action" value={String.capitalize(@job.action)} />
                  <.detail_row label="Resource" value={String.upcase(@job.resource)} />
                  <.detail_row label="Network" value={String.capitalize(@job.network)} />
                  <.detail_row label="Block" value={@job.block_number || "latest"} />
                  <%= if @job.proof_type do %>
                    <.detail_row label="Proof Type" value={String.capitalize(@job.proof_type)} />
                  <% end %>
                  <.detail_row label="Cache" value={String.capitalize(@job.cache_level || "on")} />
                  <%= if @job.ethrex_branch do %>
                    <.detail_row label="Ethrex Branch" value={@job.ethrex_branch} />
                  <% end %>
                </div>
              </div>
            </div>
            
    <!-- Timing -->
            <div class="card bg-base-200 border border-base-300">
              <div class="card-body">
                <h2 class="card-title text-lg">
                  <span class="hero-clock w-5 h-5"></span> Timing
                </h2>
                <div class="space-y-3 mt-2">
                  <div class="flex justify-between text-sm">
                    <span class="text-base-content/60">Created</span>
                    <span
                      class="font-medium"
                      phx-hook="LocalTime"
                      id="job-created-time"
                      data-timestamp={NaiveDateTime.to_iso8601(@job.inserted_at)}
                    >
                      {format_datetime(@job.inserted_at)}
                    </span>
                  </div>
                  <%= if @job.updated_at != @job.inserted_at do %>
                    <div class="flex justify-between text-sm">
                      <span class="text-base-content/60">Updated</span>
                      <span
                        class="font-medium"
                        phx-hook="LocalTime"
                        id="job-updated-time"
                        data-timestamp={NaiveDateTime.to_iso8601(@job.updated_at)}
                      >
                        {format_datetime(@job.updated_at)}
                      </span>
                    </div>
                  <% end %>
                </div>
              </div>
            </div>
            
    <!-- RPC URL -->
            <div class="card bg-base-200 border border-base-300">
              <div class="card-body">
                <h2 class="card-title text-lg">
                  <span class="hero-globe-alt w-5 h-5"></span> RPC Endpoint
                </h2>
                <div class="bg-base-300/50 rounded-lg p-3 mt-2 font-mono text-xs break-all">
                  {@job.rpc_url}
                </div>
              </div>
            </div>
            
    <!-- Actions -->
            <div class="card bg-base-200 border border-base-300">
              <div class="card-body">
                <h2 class="card-title text-lg">
                  <span class="hero-bolt w-5 h-5"></span> Actions
                </h2>
                <div class="flex flex-col gap-2 mt-2">
                  <a href="/" class="btn btn-outline btn-sm">
                    <span class="hero-plus w-4 h-4"></span> New Job
                  </a>
                  <button phx-click="rerun" class="btn btn-outline btn-primary btn-sm">
                    <span class="hero-arrow-path w-4 h-4"></span> Rerun Job
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </main>
      
    <!-- Footer -->
      <footer class="footer footer-center p-6 bg-base-200 text-base-content/60">
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
        "pending" -> {"bg-neutral/50", "text-neutral-content", "Pending"}
        "queued" -> {"bg-neutral/50", "text-neutral-content", "Queued"}
        "running" -> {"bg-primary/20", "text-primary", "Running"}
        "completed" -> {"bg-success/20", "text-success", "Completed"}
        "failed" -> {"bg-error/20", "text-error", "Failed"}
        "cancelled" -> {"bg-warning/20", "text-warning", "Cancelled"}
        _ -> {"bg-neutral/50", "text-neutral-content", assigns.status}
      end

    assigns = assign(assigns, bg_class: bg_class, text_class: text_class, label: label)

    ~H"""
    <div class={["badge badge-lg gap-2 py-3", @bg_class, @text_class]}>
      <span class={"status-dot status-dot-#{@status}"}></span>
      {@label}
    </div>
    """
  end

  attr :label, :string, required: true
  attr :value, :any, required: true

  defp detail_row(assigns) do
    ~H"""
    <div class="flex justify-between text-sm">
      <span class="text-base-content/60">{@label}</span>
      <span class="font-medium">{@value}</span>
    </div>
    """
  end

  attr :label, :string, required: true
  attr :value, :string, required: true

  defp result_metric(assigns) do
    ~H"""
    <div class="bg-base-300/50 rounded-lg p-4">
      <div class="text-xs text-base-content/60 uppercase tracking-wide mb-1">{@label}</div>
      <div class="text-xl font-bold text-primary">{@value}</div>
    </div>
    """
  end

  # Event Handlers

  @impl true
  def handle_event("toggle_auto_scroll", _params, socket) do
    {:noreply, assign(socket, :auto_scroll, !socket.assigns.auto_scroll)}
  end

  @impl true
  def handle_event("cancel", _params, socket) do
    JobQueue.cancel_job(socket.assigns.job.id)
    {:noreply, socket}
  end

  @impl true
  def handle_event("rerun", _params, socket) do
    job = socket.assigns.job

    attrs = %{
      zkvm: job.zkvm,
      action: job.action,
      resource: job.resource,
      proof_type: job.proof_type,
      network: job.network,
      rpc_url: job.rpc_url,
      cache_level: job.cache_level,
      ethrex_branch: job.ethrex_branch,
      block_number: job.block_number
    }

    case JobQueue.submit_job(attrs) do
      {:ok, new_job} ->
        {:noreply,
         socket
         |> put_flash(:info, "New job created!")
         |> push_navigate(to: ~p"/jobs/#{new_job.id}")}

      {:error, _changeset} ->
        {:noreply, put_flash(socket, :error, "Failed to create job")}
    end
  end

  # PubSub Handlers

  @impl true
  def handle_info({:job_log, _job_id, line}, socket) do
    {:noreply, assign(socket, :logs, socket.assigns.logs ++ [line])}
  end

  @impl true
  def handle_info({:job_status, _job_id, _status}, socket) do
    job = Jobs.get_job!(socket.assigns.job.id)
    {:noreply, assign(socket, :job, job)}
  end

  @impl true
  def handle_info(_msg, socket) do
    {:noreply, socket}
  end

  # Helpers

  defp short_id(id) do
    String.slice(id, 0, 8)
  end

  defp log_line_class(line) do
    line_lower = String.downcase(line)

    cond do
      String.contains?(line_lower, "error") -> "log-line-error"
      String.contains?(line_lower, "warn") -> "log-line-warn"
      String.contains?(line_lower, "info") -> "log-line-info"
      String.contains?(line_lower, "debug") -> "log-line-debug"
      String.contains?(line_lower, "success") -> "log-line-success"
      true -> ""
    end
  end

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

  defp format_duration(_), do: "-"

  defp format_number(n) when is_integer(n) do
    n
    |> Integer.to_string()
    |> String.graphemes()
    |> Enum.reverse()
    |> Enum.chunk_every(3)
    |> Enum.map(&Enum.reverse/1)
    |> Enum.reverse()
    |> Enum.join(",")
  end

  defp format_number(_), do: "-"

  defp format_datetime(datetime) do
    Calendar.strftime(datetime, "%Y-%m-%d %H:%M:%S")
  end
end
