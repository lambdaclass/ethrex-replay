defmodule EthrexReplayWeb.System.HardwareInfo do
  @moduledoc """
  Detects hardware information about the system (CPU, GPU, RAM).
  """

  @doc """
  Returns all hardware information.
  """
  def get_all do
    %{
      cpu: get_cpu_info(),
      gpu: get_gpu_info(),
      ram: get_ram_info(),
      os: get_os_info()
    }
  end

  @doc """
  Gets CPU information.
  """
  def get_cpu_info do
    {model, cores} =
      case :os.type() do
        {:unix, :darwin} -> get_macos_cpu()
        {:unix, _} -> get_linux_cpu()
        {:win32, _} -> get_windows_cpu()
        _ -> {"Unknown", 0}
      end

    %{
      model: model,
      cores: cores
    }
  end

  @doc """
  Gets GPU information.
  Only NVIDIA GPUs (detected via nvidia-smi) are considered available for proving.
  """
  def get_gpu_info do
    # Only NVIDIA GPUs with nvidia-smi are supported for GPU proving
    case get_nvidia_gpu() do
      {:ok, info} ->
        info

      :error ->
        # No CUDA-capable GPU available for proving
        %{
          model: "No GPU available",
          memory_mb: nil,
          memory_gb: nil,
          available: false,
          type: :none
        }
    end
  end

  @doc """
  Gets RAM information.
  """
  def get_ram_info do
    total_bytes =
      case :os.type() do
        {:unix, :darwin} -> get_macos_ram()
        {:unix, _} -> get_linux_ram()
        {:win32, _} -> get_windows_ram()
        _ -> 0
      end

    %{
      total_bytes: total_bytes,
      total_gb: Float.round(total_bytes / (1024 * 1024 * 1024), 1)
    }
  end

  @doc """
  Gets OS information.
  """
  def get_os_info do
    {os_family, os_name} = :os.type()

    version =
      case System.cmd("uname", ["-r"], stderr_to_stdout: true) do
        {output, 0} -> String.trim(output)
        _ -> "Unknown"
      end

    %{
      family: os_family,
      name: os_name,
      version: version
    }
  end

  # Private functions

  defp get_macos_cpu do
    model =
      case System.cmd("sysctl", ["-n", "machdep.cpu.brand_string"], stderr_to_stdout: true) do
        {output, 0} -> String.trim(output)
        _ -> "Unknown"
      end

    cores =
      case System.cmd("sysctl", ["-n", "hw.ncpu"], stderr_to_stdout: true) do
        {output, 0} ->
          case Integer.parse(String.trim(output)) do
            {n, _} -> n
            :error -> 0
          end

        _ ->
          0
      end

    {model, cores}
  end

  defp get_linux_cpu do
    model =
      case File.read("/proc/cpuinfo") do
        {:ok, content} ->
          content
          |> String.split("\n")
          |> Enum.find_value("Unknown", fn line ->
            case String.split(line, ":") do
              ["model name" <> _, value] -> String.trim(value)
              _ -> nil
            end
          end)

        _ ->
          "Unknown"
      end

    cores = System.schedulers_online()

    {model, cores}
  end

  defp get_windows_cpu do
    model =
      case System.cmd("wmic", ["cpu", "get", "name"], stderr_to_stdout: true) do
        {output, 0} ->
          output
          |> String.split("\n")
          |> Enum.at(1, "Unknown")
          |> String.trim()

        _ ->
          "Unknown"
      end

    cores = System.schedulers_online()

    {model, cores}
  end

  defp get_nvidia_gpu do
    # First check if nvidia-smi exists to avoid raising :enoent
    case System.find_executable("nvidia-smi") do
      nil ->
        :error

      _path ->
        case System.cmd(
               "nvidia-smi",
               ["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"],
               stderr_to_stdout: true
             ) do
          {output, 0} ->
            case String.split(String.trim(output), ", ") do
              [name, memory] ->
                memory_mb =
                  case Integer.parse(memory) do
                    {n, _} -> n
                    :error -> 0
                  end

                {:ok,
                 %{
                   model: String.trim(name),
                   memory_mb: memory_mb,
                   memory_gb: Float.round(memory_mb / 1024, 1),
                   available: true,
                   type: :nvidia
                 }}

              _ ->
                :error
            end

          _ ->
            :error
        end
    end
  end

  defp get_macos_ram do
    case System.cmd("sysctl", ["-n", "hw.memsize"], stderr_to_stdout: true) do
      {output, 0} ->
        case Integer.parse(String.trim(output)) do
          {bytes, _} -> bytes
          :error -> 0
        end

      _ ->
        0
    end
  end

  defp get_linux_ram do
    case File.read("/proc/meminfo") do
      {:ok, content} ->
        content
        |> String.split("\n")
        |> Enum.find_value(0, fn line ->
          case String.split(line, ~r/\s+/) do
            ["MemTotal:", kb, "kB"] ->
              case Integer.parse(kb) do
                {n, _} -> n * 1024
                :error -> nil
              end

            _ ->
              nil
          end
        end)

      _ ->
        0
    end
  end

  defp get_windows_ram do
    case System.cmd("wmic", ["computersystem", "get", "totalphysicalmemory"],
           stderr_to_stdout: true
         ) do
      {output, 0} ->
        output
        |> String.split("\n")
        |> Enum.at(1, "0")
        |> String.trim()
        |> Integer.parse()
        |> case do
          {bytes, _} -> bytes
          :error -> 0
        end

      _ ->
        0
    end
  end
end
