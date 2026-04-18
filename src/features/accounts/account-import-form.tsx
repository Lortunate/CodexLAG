import { useState, type FormEvent } from "react";

import { Button } from "../../components/ui/button";
import type { OfficialAccountImportInput } from "../../lib/types";

interface ProviderOption {
  label: string;
  value: string;
}

interface AccountImportFormProps {
  errorMessage: string | null;
  isSubmitting: boolean;
  onClose: () => void;
  onSubmit: (input: OfficialAccountImportInput) => Promise<boolean>;
  providerOptions: ProviderOption[];
}

interface AccountImportDraft {
  account_id: string;
  account_identity: string;
  auth_mode: string;
  name: string;
  provider: string;
  session_credential_ref: string;
  token_credential_ref: string;
}

function createInitialDraft(defaultProvider: string): AccountImportDraft {
  return {
    account_id: "",
    account_identity: "",
    auth_mode: "",
    name: "",
    provider: defaultProvider,
    session_credential_ref: "",
    token_credential_ref: "",
  };
}

const fieldClassName =
  "w-full rounded-xl border border-border bg-background/80 px-3 py-2.5 text-sm text-foreground shadow-sm outline-none transition focus:border-primary focus:ring-2 focus:ring-primary/20";

export function AccountImportForm({
  errorMessage,
  isSubmitting,
  onClose,
  onSubmit,
  providerOptions,
}: AccountImportFormProps) {
  const initialProvider = providerOptions[0]?.value ?? "openai";
  const [draft, setDraft] = useState<AccountImportDraft>(() => createInitialDraft(initialProvider));

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const didSave = await onSubmit({
      account_id: draft.account_id.trim(),
      account_identity: draft.account_identity.trim() || null,
      auth_mode: draft.auth_mode.trim() || null,
      name: draft.name.trim(),
      provider: draft.provider.trim(),
      session_credential_ref: draft.session_credential_ref.trim(),
      token_credential_ref: draft.token_credential_ref.trim(),
    });

    if (didSave) {
      setDraft(createInitialDraft(draft.provider));
    }
  }

  return (
    <form className="space-y-6" onSubmit={handleSubmit}>
      <div className="grid gap-4 md:grid-cols-2">
        <label className="space-y-2 text-sm font-medium text-foreground">
          Display name
          <input
            required
            className={fieldClassName}
            name="name"
            placeholder="OpenAI Research Pro"
            value={draft.name}
            onChange={(event) => setDraft((current) => ({ ...current, name: event.target.value }))}
          />
        </label>

        <label className="space-y-2 text-sm font-medium text-foreground">
          Provider
          <select
            className={fieldClassName}
            name="provider"
            value={draft.provider}
            onChange={(event) =>
              setDraft((current) => ({ ...current, provider: event.target.value }))
            }
          >
            {providerOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>

        <label className="space-y-2 text-sm font-medium text-foreground">
          Account ID
          <input
            required
            className={fieldClassName}
            name="account_id"
            placeholder="prod-openai-primary"
            value={draft.account_id}
            onChange={(event) =>
              setDraft((current) => ({ ...current, account_id: event.target.value }))
            }
          />
        </label>

        <label className="space-y-2 text-sm font-medium text-foreground">
          Auth mode
          <input
            className={fieldClassName}
            name="auth_mode"
            placeholder="browser_oauth_pkce"
            value={draft.auth_mode}
            onChange={(event) =>
              setDraft((current) => ({ ...current, auth_mode: event.target.value }))
            }
          />
        </label>

        <label className="space-y-2 text-sm font-medium text-foreground">
          Session credential ref
          <input
            required
            className={fieldClassName}
            name="session_credential_ref"
            placeholder="cred://sessions/openai-primary"
            value={draft.session_credential_ref}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                session_credential_ref: event.target.value,
              }))
            }
          />
        </label>

        <label className="space-y-2 text-sm font-medium text-foreground">
          Token credential ref
          <input
            required
            className={fieldClassName}
            name="token_credential_ref"
            placeholder="cred://tokens/openai-primary"
            value={draft.token_credential_ref}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                token_credential_ref: event.target.value,
              }))
            }
          />
        </label>
      </div>

      <label className="space-y-2 text-sm font-medium text-foreground">
        Account identity
        <input
          className={fieldClassName}
          name="account_identity"
          placeholder="team:research / org:platform"
          value={draft.account_identity}
          onChange={(event) =>
            setDraft((current) => ({ ...current, account_identity: event.target.value }))
          }
        />
      </label>

      <div className="rounded-2xl border border-border/80 bg-background/60 p-4">
        <p className="text-sm font-semibold text-foreground">Detected tags and routing notes</p>
        <div className="mt-3 space-y-2 text-sm text-muted-foreground">
          <p>Plan tags are inferred from token diagnostics after the next health refresh.</p>
          <p>Credential material stays referenced by secret handles and never appears in this UI.</p>
          <p>Routing priority remains policy-driven, so account imports stay focused on identity and auth.</p>
        </div>
      </div>

      {errorMessage ? (
        <div
          role="alert"
          className="rounded-2xl border border-destructive/50 bg-destructive/10 px-4 py-3 text-sm text-destructive-foreground"
        >
          {errorMessage}
        </div>
      ) : null}

      <div className="flex flex-col-reverse gap-3 border-t border-border/80 pt-5 sm:flex-row sm:justify-end">
        <Button disabled={isSubmitting} type="button" variant="outline" onClick={onClose}>
          Cancel
        </Button>
        <Button disabled={isSubmitting} type="submit">
          {isSubmitting ? "Adding official account..." : "Add official account"}
        </Button>
      </div>
    </form>
  );
}
