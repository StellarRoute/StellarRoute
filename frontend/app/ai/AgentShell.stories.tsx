import type { Story } from '@ladle/react';
import '@/app/globals.css';
import * as React from 'react';

// Inline stub types so this story file works on main before the agent
// components land. When the agent branch merges, replace these with
// direct imports from '@/app/ai/...'.

type AgentHealthState = 'available' | 'off' | 'checking';

interface ParsedAgentIntent {
  kind: string;
  summary?: string;
  amount?: string;
}

function AgentStatusChipStub({ status }: { status: AgentHealthState }) {
  const isAvailable = status === 'available';
  return (
    <div
      data-testid="agent-status-chip"
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 6,
        padding: '2px 10px',
        borderRadius: 9999,
        fontSize: 12,
        fontWeight: 500,
        border: `1px solid ${isAvailable ? '#10b98133' : '#e5e7eb'}`,
        background: isAvailable ? '#10b98110' : '#f3f4f6',
        color: isAvailable ? '#10b981' : '#6b7280',
      }}
    >
      <span
        style={{
          width: 6,
          height: 6,
          borderRadius: '50%',
          background: isAvailable ? '#10b981' : '#9ca3af',
        }}
      />
      <span data-testid="agent-status-label">{status}</span>
    </div>
  );
}

function IntentPreviewCardStub({
  intent,
  onCancel,
}: {
  intent: ParsedAgentIntent;
  onCancel?: () => void;
}) {
  const [confirmed, setConfirmed] = React.useState(false);
  return (
    <div
      data-testid="intent-preview-card"
      style={{
        border: '1px solid #e5e7eb',
        borderRadius: 8,
        padding: 16,
        background: '#fff',
        maxWidth: 480,
      }}
    >
      <div style={{ fontSize: 10, textTransform: 'uppercase', letterSpacing: 1, color: '#6b7280' }}>
        {intent.kind} preview
      </div>
      <p data-testid="intent-description" style={{ margin: '8px 0', fontWeight: 500 }}>
        {intent.summary ?? `${intent.kind} request`}
      </p>
      <div style={{ display: 'flex', gap: 8 }}>
        <button
          data-testid="confirm-btn"
          disabled={confirmed}
          onClick={() => setConfirmed(true)}
          style={{
            padding: '6px 14px',
            borderRadius: 6,
            border: 'none',
            background: confirmed ? '#9ca3af' : '#6366f1',
            color: '#fff',
            cursor: confirmed ? 'default' : 'pointer',
          }}
        >
          {confirmed ? 'Confirmed' : 'Confirm'}
        </button>
        <button
          data-testid="cancel-btn"
          onClick={onCancel}
          style={{
            padding: '6px 14px',
            borderRadius: 6,
            border: '1px solid #e5e7eb',
            background: '#fff',
            cursor: 'pointer',
          }}
        >
          Cancel
        </button>
      </div>
    </div>
  );
}

function AgentShellStub({
  statusState,
  activeIntent,
  composerDisabled,
}: {
  statusState: AgentHealthState;
  activeIntent?: ParsedAgentIntent;
  composerDisabled?: boolean;
}) {
  const [intent, setIntent] = React.useState<ParsedAgentIntent | undefined>(activeIntent);
  return (
    <div style={{ maxWidth: 700, margin: '0 auto', padding: 32 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 24 }}>
        <div>
          <h1 style={{ fontSize: 24, fontWeight: 700, margin: 0 }}>AI Agent</h1>
          <p style={{ color: '#6b7280', fontSize: 14, marginTop: 4 }}>Non-custodial trading assistant.</p>
        </div>
        <AgentStatusChipStub status={statusState} />
      </div>

      {intent && (
        <IntentPreviewCardStub intent={intent} onCancel={() => setIntent(undefined)} />
      )}

      {!composerDisabled && (
        <div style={{ display: 'flex', gap: 8, marginTop: 24 }}>
          <input
            data-testid="agent-chat-input"
            placeholder="Ask the agent…"
            style={{ flex: 1, padding: '8px 12px', borderRadius: 6, border: '1px solid #e5e7eb' }}
            readOnly
          />
          <button
            data-testid="agent-chat-submit"
            style={{ padding: '8px 16px', borderRadius: 6, background: '#6366f1', color: '#fff', border: 'none' }}
          >
            Send
          </button>
        </div>
      )}
    </div>
  );
}

function DisabledShell() {
  return (
    <div style={{ maxWidth: 700, margin: '0 auto', padding: 32 }}>
      <div
        data-testid="ai-page-disabled"
        style={{
          border: '1px solid #e5e7eb',
          borderRadius: 8,
          padding: 32,
          textAlign: 'center',
        }}
      >
        <h1 style={{ fontSize: 24, fontWeight: 700 }}>AI Agent Unavailable</h1>
        <p style={{ color: '#6b7280', fontSize: 14, marginTop: 8 }}>
          The AI assistant is currently disabled on this deployment.
        </p>
      </div>
    </div>
  );
}

/** Agent shell with no input yet — status chip shows "available". */
export const Empty: Story = () => (
  <AgentShellStub statusState="available" />
);
Empty.storyName = 'Empty — Idle composer';

/** Agent shell showing a send intent preview card. */
export const WithPreview: Story = () => (
  <AgentShellStub
    statusState="available"
    activeIntent={{ kind: 'send', amount: '1', summary: 'Send 1 XLM to GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN' }}
  />
);
WithPreview.storyName = 'Preview — Intent card visible';

/** Agent shell in the clarification state (chip checking). */
export const Clarification: Story = () => (
  <AgentShellStub statusState="checking" />
);
Clarification.storyName = 'Clarification — Health checking';

/** Agent shell in disabled state — flag off, no composer actions call fetch. */
export const Disabled: Story = () => <DisabledShell />;
Disabled.storyName = 'Disabled — Flag off';
