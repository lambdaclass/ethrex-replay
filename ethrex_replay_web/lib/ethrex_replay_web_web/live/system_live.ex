defmodule EthrexReplayWebWeb.SystemLive do
  @moduledoc """
  LiveView for viewing system hardware information.
  """
  use EthrexReplayWebWeb, :live_view

  alias EthrexReplayWeb.System.HardwareInfo
  alias EthrexReplayWeb.Job

  @impl true
  def mount(_params, _session, socket) do
    hardware_info = HardwareInfo.get_all()

    {:ok,
     socket
     |> assign(:page_title, "System Info")
     |> assign(:hardware_info, hardware_info)}
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
                <path d="M12 1.5l-9 5.25v10.5l9 5.25 9-5.25V6.75L12 1.5zm0 2.5l6.5 3.75L12 11.5 5.5 7.75 12 4zm-7 5.5l6 3.5v7l-6-3.5v-7zm14 0v7l-6 3.5v-7l6-3.5z"/>
              </svg>
              <span>Ethrex Replay</span>
            </a>
          </div>
          <div class="flex-none">
            <ul class="menu menu-horizontal px-1 gap-2">
              <li><a href="/">Dashboard</a></li>
              <li><a href="/history">History</a></li>
              <li><a href="/system" class="text-primary">System</a></li>
            </ul>
          </div>
        </div>
      </nav>

      <main class="container mx-auto px-4 py-8 flex-1">
        <!-- Header -->
        <div class="mb-8">
          <h1 class="text-3xl font-bold">System Information</h1>
          <p class="text-base-content/60 mt-1">Hardware specifications and supported configurations</p>
        </div>

        <div class="grid md:grid-cols-2 gap-6">
          <!-- CPU Card -->
          <div class="card bg-base-200 border border-base-300 card-hover">
            <div class="card-body">
              <div class="flex items-center gap-3 mb-4">
                <div class="p-3 bg-primary/10 rounded-lg">
                  <span class="hero-cpu-chip w-8 h-8 text-primary"></span>
                </div>
                <div>
                  <h2 class="card-title">CPU</h2>
                  <p class="text-base-content/60 text-sm">Central Processing Unit</p>
                </div>
              </div>

              <div class="space-y-4">
                <div>
                  <div class="text-xs text-base-content/50 uppercase tracking-wide mb-1">Model</div>
                  <div class="font-medium">{@hardware_info.cpu.model}</div>
                </div>
                <div>
                  <div class="text-xs text-base-content/50 uppercase tracking-wide mb-1">Cores</div>
                  <div class="metric-value">{@hardware_info.cpu.cores}</div>
                </div>
              </div>
            </div>
          </div>

          <!-- GPU Card -->
          <div class="card bg-base-200 border border-base-300 card-hover">
            <div class="card-body">
              <div class="flex items-center gap-3 mb-4">
                <div class="p-3 bg-primary/10 rounded-lg">
                  <span class="hero-square-3-stack-3d w-8 h-8 text-primary"></span>
                </div>
                <div>
                  <h2 class="card-title">GPU</h2>
                  <p class="text-base-content/60 text-sm">Graphics Processing Unit</p>
                </div>
              </div>

              <div class="space-y-4">
                <div>
                  <div class="text-xs text-base-content/50 uppercase tracking-wide mb-1">Model</div>
                  <div class="font-medium">{@hardware_info.gpu.model}</div>
                </div>
                <%= if @hardware_info.gpu[:memory_gb] do %>
                  <div>
                    <div class="text-xs text-base-content/50 uppercase tracking-wide mb-1">VRAM</div>
                    <div class="metric-value">{@hardware_info.gpu.memory_gb} <span class="text-lg text-base-content/60">GB</span></div>
                  </div>
                <% end %>
                <div>
                  <div class="text-xs text-base-content/50 uppercase tracking-wide mb-1">Status</div>
                  <%= if @hardware_info.gpu[:available] do %>
                    <span class="badge badge-success gap-1">
                      <span class="hero-check w-4 h-4"></span>
                      Available
                    </span>
                  <% else %>
                    <span class="badge badge-warning gap-1">
                      <span class="hero-exclamation-triangle w-4 h-4"></span>
                      Not Detected
                    </span>
                  <% end %>
                </div>
              </div>
            </div>
          </div>

          <!-- RAM Card -->
          <div class="card bg-base-200 border border-base-300 card-hover">
            <div class="card-body">
              <div class="flex items-center gap-3 mb-4">
                <div class="p-3 bg-primary/10 rounded-lg">
                  <span class="hero-server-stack w-8 h-8 text-primary"></span>
                </div>
                <div>
                  <h2 class="card-title">Memory</h2>
                  <p class="text-base-content/60 text-sm">System RAM</p>
                </div>
              </div>

              <div class="space-y-4">
                <div>
                  <div class="text-xs text-base-content/50 uppercase tracking-wide mb-1">Total</div>
                  <div class="metric-value">{@hardware_info.ram.total_gb} <span class="text-lg text-base-content/60">GB</span></div>
                </div>
              </div>
            </div>
          </div>

          <!-- OS Card -->
          <div class="card bg-base-200 border border-base-300 card-hover">
            <div class="card-body">
              <div class="flex items-center gap-3 mb-4">
                <div class="p-3 bg-primary/10 rounded-lg">
                  <span class="hero-computer-desktop w-8 h-8 text-primary"></span>
                </div>
                <div>
                  <h2 class="card-title">Operating System</h2>
                  <p class="text-base-content/60 text-sm">Platform Information</p>
                </div>
              </div>

              <div class="space-y-4">
                <div>
                  <div class="text-xs text-base-content/50 uppercase tracking-wide mb-1">Family</div>
                  <div class="font-medium">{format_os_family(@hardware_info.os.family)}</div>
                </div>
                <div>
                  <div class="text-xs text-base-content/50 uppercase tracking-wide mb-1">Version</div>
                  <div class="font-mono text-sm">{@hardware_info.os.version}</div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- ZKVM Support Status -->
        <div class="mt-8">
          <h2 class="text-2xl font-bold mb-4">ZKVM Support Status</h2>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
            <%= for zkvm <- Job.zkvms() do %>
              <div class="card bg-base-200 border border-base-300 card-hover">
                <div class="card-body p-4">
                  <div class="flex items-center justify-between">
                    <span class="font-bold text-lg">{String.upcase(zkvm)}</span>
                    <.zkvm_status zkvm={zkvm} />
                  </div>
                  <div class="text-xs text-base-content/50 mt-1">
                    {zkvm_description(zkvm)}
                  </div>
                </div>
              </div>
            <% end %>
          </div>
        </div>

        <!-- GPU Support -->
        <div class="mt-8">
          <h2 class="text-2xl font-bold mb-4">GPU Support by ZKVM</h2>
          <div class="card bg-base-200 border border-base-300">
            <div class="card-body">
              <p class="text-base-content/70 mb-4">
                GPU acceleration (CUDA) is available for select ZKVMs during proving:
              </p>
              <div class="overflow-x-auto">
                <table class="table">
                  <thead>
                    <tr>
                      <th>ZKVM</th>
                      <th>CPU</th>
                      <th>GPU (CUDA)</th>
                      <th>Notes</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr>
                      <td class="font-bold">SP1</td>
                      <td><span class="badge badge-success">Supported</span></td>
                      <td><span class="badge badge-success">Supported</span></td>
                      <td class="text-sm text-base-content/60">Set SP1_PROVER=cuda for GPU</td>
                    </tr>
                    <tr>
                      <td class="font-bold">RISC0</td>
                      <td><span class="badge badge-success">Supported</span></td>
                      <td><span class="badge badge-success">Supported</span></td>
                      <td class="text-sm text-base-content/60">CUDA support built-in</td>
                    </tr>
                    <tr>
                      <td class="font-bold">OpenVM</td>
                      <td><span class="badge badge-warning">Experimental</span></td>
                      <td><span class="badge badge-neutral">Not Available</span></td>
                      <td class="text-sm text-base-content/60">Work in progress</td>
                    </tr>
                    <tr>
                      <td class="font-bold">ZisK</td>
                      <td><span class="badge badge-warning">Experimental</span></td>
                      <td><span class="badge badge-neutral">Not Available</span></td>
                      <td class="text-sm text-base-content/60">Work in progress</td>
                    </tr>
                    <tr>
                      <td class="font-bold">Others</td>
                      <td><span class="badge badge-neutral">Coming Soon</span></td>
                      <td><span class="badge badge-neutral">Coming Soon</span></td>
                      <td class="text-sm text-base-content/60">Jolt, Nexus, Pico, Ziren</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </div>

        <!-- Supported Networks -->
        <div class="mt-8">
          <h2 class="text-2xl font-bold mb-4">Supported Networks</h2>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
            <%= for network <- Job.networks() do %>
              <div class="card bg-base-200 border border-base-300 card-hover">
                <div class="card-body p-4 flex flex-row items-center gap-3">
                  <div class="w-3 h-3 rounded-full bg-primary"></div>
                  <span class="font-medium">{String.capitalize(network)}</span>
                </div>
              </div>
            <% end %>
          </div>
        </div>
      </main>

      <!-- Footer -->
      <footer class="footer footer-center p-6 bg-base-200 text-base-content/60 mt-auto">
        <div>
          <p>
            Built with <span class="text-primary">ethrex</span> ·
            <a href="https://github.com/lambdaclass/ethrex-replay" class="link link-hover text-primary" target="_blank">
              GitHub
            </a>
          </p>
        </div>
      </footer>
    </div>
    """
  end

  # Components

  attr :zkvm, :string, required: true

  defp zkvm_status(assigns) do
    status =
      case assigns.zkvm do
        "sp1" -> :supported
        "risc0" -> :supported
        "openvm" -> :experimental
        "zisk" -> :experimental
        _ -> :coming_soon
      end

    assigns = assign(assigns, :status, status)

    ~H"""
    <%= case @status do %>
      <% :supported -> %>
        <span class="badge badge-success badge-sm">Supported</span>
      <% :experimental -> %>
        <span class="badge badge-warning badge-sm">Experimental</span>
      <% :coming_soon -> %>
        <span class="badge badge-neutral badge-sm">Coming Soon</span>
    <% end %>
    """
  end

  # Helpers

  defp format_os_family(:unix), do: "Unix"
  defp format_os_family(:win32), do: "Windows"
  defp format_os_family(other), do: to_string(other) |> String.capitalize()

  defp zkvm_description("sp1"), do: "Succinct Prover 1"
  defp zkvm_description("risc0"), do: "RISC Zero zkVM"
  defp zkvm_description("openvm"), do: "OpenVM by a16z"
  defp zkvm_description("zisk"), do: "ZisK by Polygon"
  defp zkvm_description("jolt"), do: "Jolt by a16z"
  defp zkvm_description("nexus"), do: "Nexus zkVM"
  defp zkvm_description("pico"), do: "Pico zkVM"
  defp zkvm_description("ziren"), do: "Ziren zkVM"
  defp zkvm_description(_), do: "Zero-Knowledge VM"
end
