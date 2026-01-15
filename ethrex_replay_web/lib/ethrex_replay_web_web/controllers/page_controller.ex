defmodule EthrexReplayWebWeb.PageController do
  use EthrexReplayWebWeb, :controller

  def home(conn, _params) do
    render(conn, :home)
  end
end
