import { useState, type FormEvent } from "react";
import type { OfficialAccountImportInput } from "../../lib/types";

interface AccountImportFormProps {
  errorMessage: string | null;
  isSubmitting: boolean;
  onSubmit: (input: OfficialAccountImportInput) => Promise<boolean>;
  successMessage: string | null;
}

interface AccountImportDraft {
  account_id: string;
  name: string;
  provider: string;
  session_credential_ref: string;
  token_credential_ref: string;
  account_identity: string;
  auth_mode: string;
}

const initialDraft: AccountImportDraft = {
  account_id: "",
  name: "",
  provider: "openai",
  session_credential_ref: "",
  token_credential_ref: "",
  account_identity: "",
  auth_mode: "",
};

export function AccountImportForm({
  errorMessage,
  isSubmitting,
  onSubmit,
  successMessage,
}: AccountImportFormProps) {
  const [draft, setDraft] = useState<AccountImportDraft>(initialDraft);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const didSave = await onSubmit({
      account_id: draft.account_id.trim(),
      name: draft.name.trim(),
      provider: draft.provider.trim(),
      session_credential_ref: draft.session_credential_ref.trim(),
      token_credential_ref: draft.token_credential_ref.trim(),
      account_identity: draft.account_identity.trim() || null,
      auth_mode: draft.auth_mode.trim() || null,
    });

    if (didSave) {
      setDraft((current) => ({
        ...initialDraft,
        provider: current.provider,
      }));
    }
  }

  return (
    <section className="panel" aria-labelledby="account-import-heading">
      <h3 id="account-import-heading">Import Official Account</h3>
      <form onSubmit={handleSubmit}>
        <p>
          <label>
            Account ID
            <input
              name="account_id"
              value={draft.account_id}
              onChange={(event) =>
                setDraft((current) => ({ ...current, account_id: event.target.value }))
              }
            />
          </label>
        </p>
        <p>
          <label>
            Account Name
            <input
              name="name"
              value={draft.name}
              onChange={(event) =>
                setDraft((current) => ({ ...current, name: event.target.value }))
              }
            />
          </label>
        </p>
        <p>
          <label>
            Provider
            <input
              name="provider"
              value={draft.provider}
              onChange={(event) =>
                setDraft((current) => ({ ...current, provider: event.target.value }))
              }
            />
          </label>
        </p>
        <p>
          <label>
            Session Credential Ref
            <input
              name="session_credential_ref"
              value={draft.session_credential_ref}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  session_credential_ref: event.target.value,
                }))
              }
            />
          </label>
        </p>
        <p>
          <label>
            Token Credential Ref
            <input
              name="token_credential_ref"
              value={draft.token_credential_ref}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  token_credential_ref: event.target.value,
                }))
              }
            />
          </label>
        </p>
        <p>
          <label>
            Account Identity (optional)
            <input
              name="account_identity"
              value={draft.account_identity}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  account_identity: event.target.value,
                }))
              }
            />
          </label>
        </p>
        <p>
          <label>
            Auth Mode (optional)
            <input
              name="auth_mode"
              value={draft.auth_mode}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  auth_mode: event.target.value,
                }))
              }
            />
          </label>
        </p>
        <button type="submit" disabled={isSubmitting}>
          Import account
        </button>
      </form>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      {successMessage ? <p>{successMessage}</p> : null}
    </section>
  );
}
