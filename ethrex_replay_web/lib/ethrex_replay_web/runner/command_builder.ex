defmodule EthrexReplayWeb.Runner.CommandBuilder do
  @moduledoc """
  Builds cargo commands for ethrex-replay execution.
  """

  alias EthrexReplayWeb.Job

  @doc """
  Builds the full cargo command for a job.
  Returns {:ok, {executable, args}} or {:error, reason}.
  """
  def build(%Job{} = job) do
    with :ok <- validate_job(job) do
      executable = System.find_executable("cargo")

      if executable do
        {:ok, {executable, build_args(job)}}
      else
        {:error, "cargo executable not found"}
      end
    end
  end

  @doc """
  Returns a preview string of the command that would be executed.
  """
  def preview(params) when is_map(params) do
    job = struct(Job, atomize_keys(params))
    build_command_string(job)
  end

  def preview(%Job{} = job) do
    build_command_string(job)
  end

  defp build_command_string(job) do
    args = build_args(job)
    "cargo #{Enum.join(args, " ")}"
  end

  defp build_args(job) do
    base_args = ["run", "--release"]

    features = build_features(job)
    feature_args = if features != "", do: ["--features", features], else: []

    # Separator between cargo args and ethrex-replay args
    separator = ["--"]

    # ethrex-replay command and arguments
    replay_args = build_replay_args(job)

    base_args ++ feature_args ++ separator ++ replay_args
  end

  defp build_features(job) do
    features = []

    # Add zkvm feature if specified and not a vanilla execution
    features =
      if job.zkvm && job.zkvm != "" do
        [job.zkvm | features]
      else
        features
      end

    # Add gpu feature only for proving (not needed for execution)
    features =
      if job.resource == "gpu" && job.action == "prove" do
        ["gpu" | features]
      else
        features
      end

    Enum.join(Enum.reverse(features), ",")
  end

  defp build_replay_args(job) do
    args = []

    # Command type: block
    args = ["block" | args]

    # Block number (if specified)
    args =
      if is_integer(job.block_number) && job.block_number > 0 do
        [Integer.to_string(job.block_number) | args]
      else
        args
      end

    # --zkvm option (prepend value first, then flag, so after reverse: --zkvm value)
    args =
      if job.zkvm && job.zkvm != "" do
        [job.zkvm, "--zkvm" | args]
      else
        args
      end

    # --action option
    args =
      if job.action && job.action != "" do
        [job.action, "--action" | args]
      else
        args
      end

    # --resource option
    args =
      if job.resource && job.resource != "" do
        [job.resource, "--resource" | args]
      else
        args
      end

    # --proof option (only for prove action)
    args =
      if job.action == "prove" && job.proof_type && job.proof_type != "" do
        [job.proof_type, "--proof" | args]
      else
        args
      end

    # --network option
    args =
      if job.network && job.network != "" do
        [job.network, "--network" | args]
      else
        args
      end

    # --rpc-url option
    args =
      if job.rpc_url && job.rpc_url != "" do
        [job.rpc_url, "--rpc-url" | args]
      else
        args
      end

    # --cache-level option
    args =
      if job.cache_level && job.cache_level != "" do
        [job.cache_level, "--cache-level" | args]
      else
        args
      end

    Enum.reverse(args)
  end

  defp validate_job(job) do
    cond do
      is_nil(job.rpc_url) or job.rpc_url == "" ->
        {:error, "RPC URL is required"}

      is_nil(job.zkvm) or job.zkvm == "" ->
        {:error, "ZKVM is required"}

      is_nil(job.action) or job.action == "" ->
        {:error, "Action is required"}

      is_nil(job.resource) or job.resource == "" ->
        {:error, "Resource is required"}

      is_nil(job.network) or job.network == "" ->
        {:error, "Network is required"}

      true ->
        :ok
    end
  end

  defp atomize_keys(map) when is_map(map) do
    Map.new(map, fn
      {k, v} when is_binary(k) ->
        key =
          case k do
            "block_number" -> :block_number
            "rpc_url" -> :rpc_url
            "cache_level" -> :cache_level
            "proof_type" -> :proof_type
            other -> String.to_existing_atom(other)
          end

        {key, v}

      {k, v} ->
        {k, v}
    end)
  rescue
    ArgumentError -> map
  end
end
