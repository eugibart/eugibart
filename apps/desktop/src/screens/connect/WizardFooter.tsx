import { ReactNode } from "react";

/**
 * One footer layout for every wizard step: Cancel far left (quiet, always
 * in the same place), Back + the step's primary action(s) on the right.
 * Steps supply their primary button(s) as children so validation stays
 * local to the step.
 */
export default function WizardFooter({
  onCancel,
  onBack,
  busy = false,
  children,
}: {
  onCancel: () => void;
  onBack?: () => void;
  busy?: boolean;
  children: ReactNode;
}) {
  return (
    <div className="wizard-footer">
      <button className="btn btn-small" onClick={onCancel} disabled={busy}>
        Cancel
      </button>
      <span className="wizard-footer-spacer" aria-hidden="true" />
      {onBack && (
        <button className="btn" onClick={onBack} disabled={busy}>
          Back
        </button>
      )}
      {children}
    </div>
  );
}
