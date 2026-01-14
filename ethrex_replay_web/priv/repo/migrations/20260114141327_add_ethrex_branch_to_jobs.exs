defmodule EthrexReplayWeb.Repo.Migrations.AddEthrexBranchToJobs do
  use Ecto.Migration

  def change do
    alter table(:jobs) do
      add :ethrex_branch, :string
    end
  end
end
