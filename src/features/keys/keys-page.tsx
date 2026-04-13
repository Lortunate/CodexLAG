import { useEffect, useState } from "react";
import {
  createPlatformKey,
  disablePlatformKey,
  enablePlatformKey,
  listPlatformKeys,
} from "../../lib/tauri";
import type { CreatePlatformKeyInput, PlatformKeyInventoryEntry } from "../../lib/types";
import { KeyManagementPanel } from "./key-management-panel";

export function KeysPage() {
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [panelErrorMessage, setPanelErrorMessage] = useState<string | null>(null);
  const [panelSuccessMessage, setPanelSuccessMessage] = useState<string | null>(null);
  const [keys, setKeys] = useState<PlatformKeyInventoryEntry[]>([]);
  const [isCreatingKey, setIsCreatingKey] = useState(false);
  const [keyActionId, setKeyActionId] = useState<string | null>(null);

  async function loadKeys(isMounted: () => boolean = () => true) {
    try {
      const nextKeys = await listPlatformKeys();
      if (isMounted()) {
        setKeys(nextKeys);
        setErrorMessage(null);
      }
    } catch {
      if (isMounted()) {
        setErrorMessage("Failed to load platform keys.");
      }
    }
  }

  useEffect(() => {
    let isMounted = true;
    void loadKeys(() => isMounted);
    return () => {
      isMounted = false;
    };
  }, []);

  async function handleCreateKey(input: CreatePlatformKeyInput) {
    if (isCreatingKey) {
      return;
    }

    setIsCreatingKey(true);
    setPanelErrorMessage(null);
    setPanelSuccessMessage(null);
    try {
      const created = await createPlatformKey(input);
      setKeys((current) => [...current.filter((key) => key.id !== created.id), created]);
      setPanelSuccessMessage(`Created key: ${created.id}`);
    } catch (error) {
      setPanelErrorMessage(error instanceof Error ? error.message : "Failed to create key.");
    } finally {
      setIsCreatingKey(false);
    }
  }

  async function handleDisableKey(keyId: string) {
    setKeyActionId(keyId);
    setPanelErrorMessage(null);
    setPanelSuccessMessage(null);
    try {
      const updated = await disablePlatformKey(keyId);
      setKeys((current) => current.map((key) => (key.id === updated.id ? updated : key)));
      setPanelSuccessMessage(`Updated key: ${updated.id}`);
    } catch (error) {
      setPanelErrorMessage(error instanceof Error ? error.message : `Failed to disable ${keyId}.`);
    } finally {
      setKeyActionId(null);
    }
  }

  async function handleEnableKey(keyId: string) {
    setKeyActionId(keyId);
    setPanelErrorMessage(null);
    setPanelSuccessMessage(null);
    try {
      const updated = await enablePlatformKey(keyId);
      setKeys((current) => current.map((key) => (key.id === updated.id ? updated : key)));
      setPanelSuccessMessage(`Updated key: ${updated.id}`);
    } catch (error) {
      setPanelErrorMessage(error instanceof Error ? error.message : `Failed to enable ${keyId}.`);
    } finally {
      setKeyActionId(null);
    }
  }

  return (
    <section aria-labelledby="keys-heading">
      <h2 id="keys-heading">Key Inventory</h2>
      <p>Track which key material is available to the gateway and how it can be routed.</p>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      <KeyManagementPanel
        errorMessage={panelErrorMessage}
        isCreating={isCreatingKey}
        keyActionId={keyActionId}
        keys={keys}
        successMessage={panelSuccessMessage}
        onCreate={handleCreateKey}
        onDisable={handleDisableKey}
        onEnable={handleEnableKey}
      />
    </section>
  );
}
