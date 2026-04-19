import { ExtensionContext, workspace, window, commands } from "vscode";

import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

async function getUmaLsPath(): Promise<string | null> {
  const configuration = workspace.getConfiguration("uma.umals");
  const umalsExePath = configuration.get<string>("path");

  if (!umalsExePath) {
    window
      .showErrorMessage("'uma.umals.path' is not configured.", "Open Settings")
      .then(async (res) => {
        if (res == "Open Settings") {
          await commands.executeCommand(
            "workbench.action.openSettings",
            "uma.umals.path",
          );
        }
      });

    return null;
  }

  return umalsExePath ?? null;
}

let client: LanguageClient;

export async function activate(context: ExtensionContext) {
  const command = await getUmaLsPath();
  if (!command) return;

  const transport = TransportKind.stdio;

  const serverOptions: ServerOptions = {
    run: { command, transport },
    debug: { command, transport },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file" }],
  };

  client = new LanguageClient("umals", "UmaLS", serverOptions, clientOptions);

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) return undefined;

  return client.stop();
}
