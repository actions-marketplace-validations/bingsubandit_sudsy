// Where the Sudsy executable and the C# worker are. Free of `vscode` imports, so that the tests
// run under plain node.
import * as path from "node:path";

export interface ServerSettings {
  /** `sudsy.importStrategy`. */
  importStrategy: "fromEnvironment" | "useBundled";
  /** `sudsy.serverPath`; empty when unset. */
  serverPath: string;
  /** `sudsy.csharpWorkerPath`; empty when unset. */
  csharpWorkerPath: string;
}

export interface ServerCommand {
  command: string;
  args: string[];
  /** Variables to add to the environment of the server. */
  env: Record<string, string>;
}

/** The environment variable `sudsy` reads the worker location from. */
export const WORKER_ENV = "SUDSY_CSHARP_WORKER";

/** The executable the extension package bundles (see scripts/stage-server.mjs). */
export function bundledServer(extensionPath: string, platform: NodeJS.Platform): string {
  return path.join(extensionPath, "server", platform === "win32" ? "sudsy.exe" : "sudsy");
}

/** `${workspaceFolder}` in a setting is the first folder of the workspace. */
export function expand(value: string, workspaceFolder: string | undefined): string {
  return workspaceFolder === undefined ? value : value.replaceAll("${workspaceFolder}", workspaceFolder);
}

/** The first `sudsy` executable in `dirs`, as a full path. */
export function findOnPath(
  dirs: string[],
  platform: NodeJS.Platform,
  exists: (file: string) => boolean,
): string | undefined {
  const name = platform === "win32" ? "sudsy.exe" : "sudsy";
  return dirs.map((d) => path.join(d, name)).find(exists);
}

/**
 * How to start the server. `fromEnvironment`: the configured executable, else the first `sudsy`
 * on PATH (`pathDirs`), else the bundled one, else plain `sudsy`. `useBundled`: the bundled one.
 * The worker needs no setting when the executable is the bundled one: `sudsy` finds
 * `csharp-worker/Sudsy.Worker.dll` beside itself.
 */
export function resolveServer(
  settings: ServerSettings,
  extensionPath: string,
  platform: NodeJS.Platform,
  exists: (file: string) => boolean,
  workspaceFolder?: string,
  pathDirs: string[] = [],
): ServerCommand {
  const bundled = bundledServer(extensionPath, platform);
  const configured = expand(settings.serverPath.trim(), workspaceFolder);
  const command =
    settings.importStrategy === "useBundled"
      ? bundled
      : configured !== ""
        ? configured
        : (findOnPath(pathDirs, platform, exists) ?? (exists(bundled) ? bundled : "sudsy"));
  const worker = expand(settings.csharpWorkerPath.trim(), workspaceFolder);
  return {
    command,
    args: ["lsp"],
    env: worker !== "" ? { [WORKER_ENV]: worker } : {},
  };
}
