defmodule EthrexReplayWebWeb.Router do
  use EthrexReplayWebWeb, :router

  pipeline :browser do
    plug :accepts, ["html"]
    plug :fetch_session
    plug :fetch_live_flash
    plug :put_root_layout, html: {EthrexReplayWebWeb.Layouts, :root}
    plug :protect_from_forgery
    plug :put_secure_browser_headers
  end

  pipeline :api do
    plug :accepts, ["json"]
  end

  scope "/", EthrexReplayWebWeb do
    pipe_through :browser

    # Main dashboard - Job submission form
    live "/", DashboardLive, :index

    # Individual job view with real-time logs
    live "/jobs/:id", JobLive, :show

    # Job history list
    live "/history", HistoryLive, :index

    # System information page
    live "/system", SystemLive, :index
  end
end
