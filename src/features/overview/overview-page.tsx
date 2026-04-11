import { DefaultKeyModeToggle } from "../default-key/default-key-mode-toggle";

export function OverviewPage() {
  return (
    <section aria-labelledby="overview-heading">
      <h2 id="overview-heading">Gateway Overview</h2>
      <p>CodexLAG manages local accounts, relays, keys, policy routing, and logs.</p>
      <DefaultKeyModeToggle />
    </section>
  );
}
