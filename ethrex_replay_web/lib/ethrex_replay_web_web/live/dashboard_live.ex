defmodule EthrexReplayWebWeb.DashboardLive do
  @moduledoc """
  Main dashboard for Ethrex Replay - Job submission and current status.
  """
  use EthrexReplayWebWeb, :live_view

  alias EthrexReplayWeb.{Job, Jobs}
  alias EthrexReplayWeb.Jobs.JobQueue
  alias EthrexReplayWeb.Runner.CommandBuilder
  alias EthrexReplayWeb.System.HardwareInfo

  @impl true
  def mount(_params, _session, socket) do
    if connected?(socket) do
      Phoenix.PubSub.subscribe(EthrexReplayWeb.PubSub, "jobs")
    end

    # Get hardware info
    hardware_info = HardwareInfo.get_all()

    # Get queue status
    queue_status = JobQueue.status()

    # Get recent jobs
    recent_jobs = Jobs.list_jobs(limit: 5)

    # Get current running job
    current_job = Jobs.get_running_job()

    # Default form values
    default_config = %{
      "zkvm" => "sp1",
      "action" => "execute",
      "resource" => "cpu",
      "proof_type" => "compressed",
      "network" => "",
      "cache_level" => "on",
      "block_number" => "",
      "rpc_url" => "",
      "ethrex_branch" => ""
    }

    {:ok,
     socket
     |> assign(:page_title, "Dashboard")
     |> assign(:hardware_info, hardware_info)
     |> assign(:queue_status, queue_status)
     |> assign(:recent_jobs, recent_jobs)
     |> assign(:current_job, current_job)
     |> assign(:form, to_form(default_config))
     |> assign(:command_preview, build_preview(default_config))
     |> assign(:submitting, false)
     |> assign(:show_advanced, false)}
  end

  @impl true
  def render(assigns) do
    ~H"""
    <div class="min-h-screen">
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
              <li><a href="/" class="text-primary">Dashboard</a></li>
              <li><a href="/history">History</a></li>
              <li><a href="/system">System</a></li>
            </ul>
          </div>
        </div>
      </nav>

      <main class="container mx-auto px-4 py-8">
        <!-- Hero Section -->
        <div class="hero-gradient rounded-2xl p-8 mb-8">
          <div class="max-w-3xl">
            <h1 class="text-4xl font-bold mb-4">
              <span class="text-gradient-primary">Proving Ethereum with Ethrex</span>
            </h1>
            <p class="text-base-content/70 text-lg">
              Execute and prove Ethereum blocks with ethrex and zkVMs.
              Generate zero-knowledge proofs of block execution.
            </p>
          </div>
        </div>
        
    <!-- Stats Grid -->
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
          <.stat_card
            label="Queue"
            value={@queue_status.pending}
            icon="hero-queue-list"
          />
          <.stat_card
            label="Running"
            value={@queue_status.running}
            icon="hero-play-circle"
            highlight={@queue_status.running > 0}
          />
          <.stat_card
            label="Completed"
            value={@queue_status.completed}
            icon="hero-check-circle"
          />
          <.stat_card
            label="Failed"
            value={@queue_status.failed}
            icon="hero-x-circle"
          />
        </div>

        <div class="grid lg:grid-cols-3 gap-8">
          <!-- Main Form Column -->
          <div class="lg:col-span-2 space-y-6">
            <!-- Configuration Card -->
            <div class="card bg-base-200 border border-base-300 card-hover">
              <div class="card-body">
                <h2 class="card-title text-xl mb-4">
                  <span class="hero-cog-6-tooth w-6 h-6"></span> Configuration
                </h2>

                <.form for={@form} phx-change="validate" phx-submit="submit" class="space-y-6">
                  <!-- Row 1: ZKVM, Action, Resource -->
                  <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <div class="form-control">
                      <label class="label">
                        <span class="label-text">ZKVM</span>
                      </label>
                      <select
                        name="zkvm"
                        class="select select-bordered w-full"
                        value={@form[:zkvm].value}
                      >
                        <%= for zkvm <- Job.zkvms() do %>
                          <% status = Job.zkvm_status(zkvm) %>
                          <option
                            value={zkvm}
                            selected={@form[:zkvm].value == zkvm}
                            disabled={status == :coming_soon}
                          >
                            {String.upcase(zkvm)}{if status == :coming_soon, do: " (Coming soon)"}{if status ==
                                                                                                        :experimental,
                                                                                                      do:
                                                                                                        " (Experimental)"}
                          </option>
                        <% end %>
                      </select>
                    </div>

                    <div class="form-control">
                      <label class="label">
                        <span class="label-text">Action</span>
                      </label>
                      <select
                        name="action"
                        class="select select-bordered w-full"
                        value={@form[:action].value}
                      >
                        <option value="execute" selected={@form[:action].value == "execute"}>
                          Execute
                        </option>
                        <option value="prove" selected={@form[:action].value == "prove"}>
                          Prove
                        </option>
                      </select>
                    </div>

                    <div class="form-control">
                      <label class="label">
                        <span class="label-text">Resource</span>
                      </label>
                      <select
                        name="resource"
                        class="select select-bordered w-full"
                        value={@form[:resource].value}
                      >
                        <option value="cpu" selected={@form[:resource].value == "cpu"}>CPU</option>
                        <option value="gpu" selected={@form[:resource].value == "gpu"}>GPU</option>
                      </select>
                    </div>
                  </div>
                  
    <!-- Row 2: Network, Block Number, Proof Type -->
                  <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <div class="form-control">
                      <label class="label">
                        <span class="label-text">Network</span>
                        <span class="label-text-alt text-base-content/50">
                          Optional - inferred from RPC
                        </span>
                      </label>
                      <select
                        name="network"
                        class="select select-bordered w-full"
                        value={@form[:network].value}
                      >
                        <option value="" selected={@form[:network].value == ""}>Auto-detect</option>
                        <%= for network <- Job.networks() do %>
                          <option value={network} selected={@form[:network].value == network}>
                            {String.capitalize(network)}
                          </option>
                        <% end %>
                      </select>
                    </div>

                    <div class="form-control">
                      <label class="label">
                        <span class="label-text">Block Number</span>
                        <span class="label-text-alt text-base-content/50">
                          Leave empty for latest
                        </span>
                      </label>
                      <input
                        type="number"
                        name="block_number"
                        value={@form[:block_number].value}
                        placeholder="latest"
                        class="input input-bordered w-full"
                        min="1"
                      />
                    </div>

                    <div class="form-control">
                      <label class="label">
                        <span class="label-text">Proof Type</span>
                        <%= if @form[:action].value == "execute" do %>
                          <span class="label-text-alt text-base-content/50">Only for prove</span>
                        <% end %>
                      </label>
                      <select
                        name="proof_type"
                        class="select select-bordered w-full"
                        value={@form[:proof_type].value}
                        disabled={@form[:action].value == "execute"}
                      >
                        <option value="compressed" selected={@form[:proof_type].value == "compressed"}>
                          Compressed
                        </option>
                        <option value="groth16" selected={@form[:proof_type].value == "groth16"}>
                          Groth16
                        </option>
                      </select>
                    </div>
                  </div>
                  
    <!-- Row 3: RPC URL -->
                  <div class="form-control">
                    <label class="label">
                      <span class="label-text">RPC URL</span>
                      <span class="label-text-alt text-base-content/50">
                        Optional if using cached blocks
                      </span>
                    </label>
                    <input
                      type="url"
                      name="rpc_url"
                      value={@form[:rpc_url].value}
                      placeholder="https://eth-mainnet.g.alchemy.com/v2/..."
                      class="input input-bordered w-full"
                    />
                  </div>
                  
    <!-- Row 4: Cache Level -->
                  <div class="form-control">
                    <label class="label">
                      <span class="label-text">Cache Level</span>
                    </label>
                    <select
                      name="cache_level"
                      class="select select-bordered w-full"
                      value={@form[:cache_level].value}
                    >
                      <option value="on" selected={@form[:cache_level].value == "on"}>On</option>
                      <option value="off" selected={@form[:cache_level].value == "off"}>Off</option>
                      <option value="failed" selected={@form[:cache_level].value == "failed"}>
                        Failed Only
                      </option>
                    </select>
                  </div>
                  
    <!-- Advanced Settings -->
                  <div class="collapse collapse-arrow bg-base-300 rounded-lg">
                    <input type="checkbox" phx-click="toggle_advanced" checked={@show_advanced} />
                    <div class="collapse-title font-medium flex items-center gap-2">
                      <span class="hero-cog-8-tooth w-5 h-5"></span> Advanced Settings
                    </div>
                    <div class="collapse-content">
                      <div class="form-control pt-2">
                        <label class="label">
                          <span class="label-text">Ethrex Branch/Commit</span>
                          <span class="label-text-alt text-base-content/50">
                            Leave empty to use repo default
                          </span>
                        </label>
                        <input
                          type="text"
                          name="ethrex_branch"
                          value={@form[:ethrex_branch].value}
                          placeholder="main"
                          class="input input-bordered w-full"
                        />
                        <label class="label">
                          <span class="label-text-alt text-base-content/50">
                            Override the ethrex dependency branch or commit hash
                          </span>
                        </label>
                      </div>
                    </div>
                  </div>
                  
    <!-- Command Preview -->
                  <div>
                    <label class="label">
                      <span class="label-text text-base-content/60">Command Preview</span>
                    </label>
                    <div class="command-preview text-sm text-primary/80">
                      {@command_preview}
                    </div>
                  </div>
                  
    <!-- Submit Button -->
                  <div class="flex justify-end">
                    <button
                      type="submit"
                      class="btn btn-primary btn-lg gap-2"
                      disabled={@submitting}
                    >
                      <%= if @submitting do %>
                        <span class="loading loading-spinner loading-sm"></span> Submitting...
                      <% else %>
                        <span class="hero-play w-5 h-5"></span> Start Job
                      <% end %>
                    </button>
                  </div>
                </.form>
              </div>
            </div>
          </div>
          
    <!-- Sidebar Column -->
          <div class="space-y-6">
            <!-- Current Job Card -->
            <%= if @current_job do %>
              <div class="card bg-base-200 border border-primary/30 glow-primary">
                <div class="card-body">
                  <h2 class="card-title text-lg">
                    <span class="status-dot status-dot-running"></span> Running Job
                  </h2>
                  <div class="space-y-2 text-sm">
                    <div class="flex justify-between">
                      <span class="text-base-content/60">ZKVM</span>
                      <span class="font-medium">{String.upcase(@current_job.zkvm)}</span>
                    </div>
                    <div class="flex justify-between">
                      <span class="text-base-content/60">Action</span>
                      <span class="font-medium">{String.capitalize(@current_job.action)}</span>
                    </div>
                    <div class="flex justify-between">
                      <span class="text-base-content/60">Block</span>
                      <span class="font-medium">{@current_job.block_number || "latest"}</span>
                    </div>
                  </div>
                  <div class="card-actions justify-end mt-4">
                    <a href={~p"/jobs/#{@current_job.id}"} class="btn btn-sm btn-outline btn-primary">
                      View Logs <span class="hero-arrow-right w-4 h-4"></span>
                    </a>
                  </div>
                </div>
              </div>
            <% end %>
            
    <!-- System Info Card -->
            <div class="card bg-base-200 border border-base-300 card-hover">
              <div class="card-body">
                <h2 class="card-title text-lg">
                  <span class="hero-cpu-chip w-5 h-5"></span> System
                </h2>
                <div class="space-y-3 text-sm">
                  <div>
                    <div class="text-base-content/60 text-xs uppercase tracking-wide mb-1">CPU</div>
                    <div class="font-medium truncate" title={@hardware_info.cpu.model}>
                      {@hardware_info.cpu.model}
                    </div>
                    <div class="text-base-content/50 text-xs">{@hardware_info.cpu.cores} cores</div>
                  </div>
                  <div>
                    <div class="text-base-content/60 text-xs uppercase tracking-wide mb-1">GPU</div>
                    <div class="font-medium truncate" title={@hardware_info.gpu.model}>
                      {@hardware_info.gpu.model}
                    </div>
                    <%= if @hardware_info.gpu[:memory_gb] do %>
                      <div class="text-base-content/50 text-xs">
                        {@hardware_info.gpu.memory_gb} GB VRAM
                      </div>
                    <% end %>
                  </div>
                  <div>
                    <div class="text-base-content/60 text-xs uppercase tracking-wide mb-1">RAM</div>
                    <div class="font-medium">{@hardware_info.ram.total_gb} GB</div>
                  </div>
                </div>
                <div class="card-actions justify-end mt-2">
                  <a href="/system" class="btn btn-sm btn-ghost">
                    More Details <span class="hero-arrow-right w-4 h-4"></span>
                  </a>
                </div>
              </div>
            </div>
            
    <!-- Recent Jobs Card -->
            <div class="card bg-base-200 border border-base-300 card-hover">
              <div class="card-body">
                <h2 class="card-title text-lg">
                  <span class="hero-clock w-5 h-5"></span> Recent Jobs
                </h2>
                <%= if @recent_jobs == [] do %>
                  <p class="text-base-content/50 text-sm">No jobs yet</p>
                <% else %>
                  <div class="space-y-2">
                    <%= for job <- @recent_jobs do %>
                      <a
                        href={~p"/jobs/#{job.id}"}
                        class="flex items-center justify-between p-2 rounded-lg hover:bg-base-300 transition-colors"
                      >
                        <div class="flex items-center gap-2">
                          <span class={"status-dot status-dot-#{job.status}"}></span>
                          <span class="text-sm font-medium">{String.upcase(job.zkvm)}</span>
                          <span class="text-xs text-base-content/50">
                            {job.block_number || "latest"}
                          </span>
                        </div>
                        <span
                          class="text-xs text-base-content/50"
                          phx-hook="LocalTime"
                          id={"job-time-#{job.id}"}
                          data-timestamp={NaiveDateTime.to_iso8601(job.inserted_at)}
                          data-format="relative"
                        >
                          {format_time(job.inserted_at)}
                        </span>
                      </a>
                    <% end %>
                  </div>
                <% end %>
                <div class="card-actions justify-end mt-2">
                  <a href="/history" class="btn btn-sm btn-ghost">
                    View All <span class="hero-arrow-right w-4 h-4"></span>
                  </a>
                </div>
              </div>
            </div>
          </div>
        </div>
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

  attr :label, :string, required: true
  attr :value, :integer, required: true
  attr :icon, :string, required: true
  attr :highlight, :boolean, default: false

  defp stat_card(assigns) do
    ~H"""
    <div class={[
      "card bg-base-200 border card-hover",
      if(@highlight, do: "border-primary/50 glow-primary", else: "border-base-300")
    ]}>
      <div class="card-body p-4">
        <div class="flex items-center gap-3">
          <span class={[
            @icon,
            "w-8 h-8",
            if(@highlight, do: "text-primary", else: "text-base-content/50")
          ]}>
          </span>
          <div>
            <div class={["metric-value", if(@highlight, do: "", else: "text-base-content")]}>
              {@value}
            </div>
            <div class="text-xs text-base-content/50 uppercase tracking-wide">{@label}</div>
          </div>
        </div>
      </div>
    </div>
    """
  end

  # Event Handlers

  @impl true
  def handle_event("toggle_advanced", _params, socket) do
    {:noreply, assign(socket, :show_advanced, !socket.assigns.show_advanced)}
  end

  @impl true
  def handle_event("validate", params, socket) do
    preview = build_preview(params)
    {:noreply, assign(socket, form: to_form(params), command_preview: preview)}
  end

  @impl true
  def handle_event("submit", params, socket) do
    socket = assign(socket, :submitting, true)

    # Parse block number
    block_number =
      case Integer.parse(params["block_number"] || "") do
        {n, _} -> n
        :error -> nil
      end

    # Parse ethrex_branch (nil if empty)
    ethrex_branch = empty_to_nil(params["ethrex_branch"])

    # Parse optional fields (empty string to nil)
    network = empty_to_nil(params["network"])
    rpc_url = empty_to_nil(params["rpc_url"])

    attrs = %{
      zkvm: params["zkvm"],
      action: params["action"],
      resource: params["resource"],
      proof_type: params["proof_type"],
      network: network,
      rpc_url: rpc_url,
      cache_level: params["cache_level"],
      ethrex_branch: ethrex_branch,
      block_number: block_number
    }

    case JobQueue.submit_job(attrs) do
      {:ok, job} ->
        {:noreply,
         socket
         |> assign(:submitting, false)
         |> put_flash(:info, "Job queued successfully!")
         |> push_navigate(to: ~p"/jobs/#{job.id}")}

      {:error, changeset} ->
        errors = format_errors(changeset)

        {:noreply,
         socket
         |> assign(:submitting, false)
         |> put_flash(:error, "Failed to create job: #{errors}")}
    end
  end

  # PubSub Handlers

  @impl true
  def handle_info({:job_created, _job}, socket) do
    queue_status = JobQueue.status()
    recent_jobs = Jobs.list_jobs(limit: 5)
    {:noreply, assign(socket, queue_status: queue_status, recent_jobs: recent_jobs)}
  end

  @impl true
  def handle_info({:job_updated, job}, socket) do
    queue_status = JobQueue.status()
    recent_jobs = Jobs.list_jobs(limit: 5)

    current_job =
      if job.status == "running", do: job, else: Jobs.get_running_job()

    {:noreply,
     assign(socket,
       queue_status: queue_status,
       recent_jobs: recent_jobs,
       current_job: current_job
     )}
  end

  @impl true
  def handle_info({:job_finished, _job_id}, socket) do
    queue_status = JobQueue.status()
    recent_jobs = Jobs.list_jobs(limit: 5)
    current_job = Jobs.get_running_job()

    {:noreply,
     assign(socket,
       queue_status: queue_status,
       recent_jobs: recent_jobs,
       current_job: current_job
     )}
  end

  @impl true
  def handle_info(_msg, socket) do
    {:noreply, socket}
  end

  # Helpers

  defp build_preview(params) do
    CommandBuilder.preview(params)
  end

  defp format_errors(changeset) do
    Ecto.Changeset.traverse_errors(changeset, fn {msg, opts} ->
      Enum.reduce(opts, msg, fn {key, value}, acc ->
        String.replace(acc, "%{#{key}}", to_string(value))
      end)
    end)
    |> Enum.map(fn {field, messages} ->
      "#{field}: #{Enum.join(messages, ", ")}"
    end)
    |> Enum.join("; ")
  end

  defp format_time(datetime) do
    now = NaiveDateTime.utc_now()
    diff = NaiveDateTime.diff(now, datetime, :second)

    cond do
      diff < 60 -> "#{diff}s ago"
      diff < 3600 -> "#{div(diff, 60)}m ago"
      diff < 86400 -> "#{div(diff, 3600)}h ago"
      true -> "#{div(diff, 86400)}d ago"
    end
  end

  defp empty_to_nil(nil), do: nil
  defp empty_to_nil(""), do: nil

  defp empty_to_nil(str) when is_binary(str) do
    case String.trim(str) do
      "" -> nil
      trimmed -> trimmed
    end
  end
end
