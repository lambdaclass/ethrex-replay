defmodule EthrexReplayWeb.Repo do
  use Ecto.Repo,
    otp_app: :ethrex_replay_web,
    adapter: Ecto.Adapters.SQLite3
end
