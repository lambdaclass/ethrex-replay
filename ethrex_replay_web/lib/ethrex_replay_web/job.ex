defmodule EthrexReplayWeb.Job do
  @moduledoc """
  Schema and changeset for proof generation jobs.
  """
  use Ecto.Schema
  import Ecto.Changeset

  @primary_key {:id, :binary_id, autogenerate: true}
  @foreign_key_type :binary_id

  @zkvms ~w(risc0 sp1 openvm zisk jolt nexus pico ziren)
  @actions ~w(execute prove)
  @resources ~w(cpu gpu)
  @proof_types ~w(compressed groth16)
  @networks ~w(mainnet sepolia hoodi holesky)
  @cache_levels ~w(on off failed)
  @statuses ~w(pending queued running completed failed cancelled)

  schema "jobs" do
    field :zkvm, :string
    field :action, :string
    field :resource, :string
    field :proof_type, :string
    field :block_number, :integer
    field :network, :string
    field :rpc_url, :string
    field :cache_level, :string
    field :ethrex_branch, :string
    field :status, :string, default: "pending"
    field :command, :string
    field :execution_time_ms, :integer
    field :proving_time_ms, :integer
    field :gas_used, :integer
    field :exit_code, :integer
    field :error, :string

    timestamps()
  end

  @doc """
  Creates a changeset for a new job.
  """
  def changeset(job, attrs) do
    job
    |> cast(attrs, [
      :zkvm,
      :action,
      :resource,
      :proof_type,
      :block_number,
      :network,
      :rpc_url,
      :cache_level,
      :ethrex_branch,
      :status,
      :command,
      :execution_time_ms,
      :proving_time_ms,
      :gas_used,
      :exit_code,
      :error
    ])
    |> validate_required([:zkvm, :action, :resource, :network, :rpc_url])
    |> validate_inclusion(:zkvm, @zkvms)
    |> validate_inclusion(:action, @actions)
    |> validate_inclusion(:resource, @resources)
    |> validate_inclusion(:proof_type, @proof_types ++ [nil])
    |> validate_inclusion(:network, @networks)
    |> validate_inclusion(:cache_level, @cache_levels ++ [nil])
    |> validate_inclusion(:status, @statuses)
    |> validate_number(:block_number, greater_than: 0)
    |> validate_rpc_url()
    |> validate_gpu_zkvm_compatibility()
  end

  defp validate_rpc_url(changeset) do
    validate_change(changeset, :rpc_url, fn :rpc_url, url ->
      case URI.parse(url) do
        %URI{scheme: scheme, host: host}
        when scheme in ["http", "https"] and is_binary(host) and host != "" ->
          []

        _ ->
          [rpc_url: "must be a valid HTTP or HTTPS URL"]
      end
    end)
  end

  defp validate_gpu_zkvm_compatibility(changeset) do
    zkvm = get_field(changeset, :zkvm)
    resource = get_field(changeset, :resource)

    gpu_compatible = ~w(sp1 risc0)

    if resource == "gpu" and zkvm not in gpu_compatible do
      add_error(changeset, :resource, "GPU is not supported for #{zkvm}")
    else
      changeset
    end
  end

  # Accessor functions for enum values
  def zkvms, do: @zkvms
  def actions, do: @actions
  def resources, do: @resources
  def proof_types, do: @proof_types
  def networks, do: @networks
  def cache_levels, do: @cache_levels
  def statuses, do: @statuses
end
