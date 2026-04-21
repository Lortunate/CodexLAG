import { useEffect, useState } from "react";
import {
  createPlatformKey,
  disablePlatformKey,
  enablePlatformKey,
  listPlatformKeys,
} from "../../lib/tauri";
import type {
  CreatePlatformKeyInput,
  CreatedPlatformKey,
  PlatformKeyInventoryEntry,
} from "../../lib/types";
import { PageHeader } from "../../components/page-header";
import { KeyManagementPanel } from "./key-management-panel";

export function KeysPage() {
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [panelErrorMessage, setPanelErrorMessage] = useState<string | null>(null);
  const [panelSuccessMessage, setPanelSuccessMessage] = useState<string | null>(null);
  const [createdKey, setCreatedKey] = useState<CreatedPlatformKey | null>(null);
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

  async function handleCreateKey(input: CreatePlatformKeyInput): Promise<boolean> {
    if (isCreatingKey) {
      return false;
    }

    setIsCreatingKey(true);
    setPanelErrorMessage(null);
    setPanelSuccessMessage(null);
    setCreatedKey(null);
    try {
      const created = await createPlatformKey(input);
      setKeys((current) => [
        ...current.filter((key) => key.id !== created.id),
        {
          id: created.id,
          name: created.name,
          policy_id: created.policy_id,
          allowed_mode: created.allowed_mode,
          enabled: created.enabled,
        },
      ]);
      setCreatedKey(created);
      setPanelSuccessMessage(`Created key: ${created.id}`);
      return true;
    } catch (error) {
      setPanelErrorMessage(error instanceof Error ? error.message : "Failed to create key.");
      return false;
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
    <section className="workspace-page" aria-labelledby="keys-heading">
      <PageHeader
        eyebrow="Gateway issuance"
        titleId="keys-heading"
        title="Platform Keys"
        description="Issue local gateway keys, bind runtime policy, and review which upstream modes each key is allowed to exercise."
      />
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      <div className="workspace-summary-strip">
        <article className="operator-callout">
          <h4>Issued keys</h4>
          <p>{keys.length} platform keys currently registered in the local gateway inventory.</p>
        </article>
        <article className="operator-callout">
          <h4>Enabled keys</h4>
          <p>{keys.filter((key) => key.enabled).length} keys can currently issue requests.</p>
        </article>
        <article className="operator-callout">
          <h4>Bound policies</h4>
          <p>{new Set(keys.map((key) => key.policy_id)).size} policy bindings are represented across key inventory.</p>
        </article>
      </div>
      <div className="workspace-grid">
        <div className="workspace-column">
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
        </div>
        <div className="workspace-column">
          <section className="panel">
            <div className="panel-heading">
              <div>
                <h3>Generated secret</h3>
                <p>Create a new platform key to reveal its one-time secret in this inspector column.</p>
              </div>
            </div>
            {createdKey ? (
              <div className="generated-secret" role="status">
                <code>{createdKey.secret}</code>
              </div>
            ) : (
              <p className="operator-empty">No newly generated secret is in view.</p>
            )}
          </section>
        </div>
      </div>
    </section>
  );
}
