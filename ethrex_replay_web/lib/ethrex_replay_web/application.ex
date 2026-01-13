defmodule EthrexReplayWeb.Application do
  # See https://hexdocs.pm/elixir/Application.html
  # for more information on OTP Applications
  @moduledoc false

  use Application

  @impl true
  def start(_type, _args) do
    children = [
      EthrexReplayWebWeb.Telemetry,
      EthrexReplayWeb.Repo,
      {Ecto.Migrator,
       repos: Application.fetch_env!(:ethrex_replay_web, :ecto_repos), skip: skip_migrations?()},
      {DNSCluster, query: Application.get_env(:ethrex_replay_web, :dns_cluster_query) || :ignore},
      {Phoenix.PubSub, name: EthrexReplayWeb.PubSub},
      # Registry for tracking job processes
      {Registry, keys: :unique, name: EthrexReplayWeb.Jobs.JobRegistry},
      # Dynamic supervisor for job processes
      EthrexReplayWeb.Jobs.JobSupervisor,
      # Job queue manager (ensures one job at a time)
      EthrexReplayWeb.Jobs.JobQueue,
      # Start to serve requests, typically the last entry
      EthrexReplayWebWeb.Endpoint
    ]

    # See https://hexdocs.pm/elixir/Supervisor.html
    # for other strategies and supported options
    opts = [strategy: :one_for_one, name: EthrexReplayWeb.Supervisor]
    Supervisor.start_link(children, opts)
  end

  # Tell Phoenix to update the endpoint configuration
  # whenever the application is updated.
  @impl true
  def config_change(changed, _new, removed) do
    EthrexReplayWebWeb.Endpoint.config_change(changed, removed)
    :ok
  end

  defp skip_migrations?() do
    # By default, sqlite migrations are run when using a release
    System.get_env("RELEASE_NAME") == nil
  end
end
