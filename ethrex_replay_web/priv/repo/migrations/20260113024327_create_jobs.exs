defmodule EthrexReplayWeb.Repo.Migrations.CreateJobs do
  use Ecto.Migration

  def change do
    create table(:jobs, primary_key: false) do
      add :id, :binary_id, primary_key: true
      add :zkvm, :string, null: false
      add :action, :string, null: false
      add :resource, :string, null: false
      add :proof_type, :string
      add :block_number, :integer
      add :network, :string, null: false
      add :rpc_url, :string, null: false
      add :cache_level, :string
      add :status, :string, null: false, default: "pending"
      add :command, :text
      add :execution_time_ms, :integer
      add :proving_time_ms, :integer
      add :gas_used, :bigint
      add :exit_code, :integer
      add :error, :text

      timestamps()
    end

    create index(:jobs, [:status])
    create index(:jobs, [:inserted_at])
  end
end
