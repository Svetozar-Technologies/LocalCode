import { useState } from 'react';
import LLMSettings from './LLMSettings';

interface SetupWizardProps {
  onComplete: () => void;
}

const styles = {
  overlay: {
    position: 'fixed' as const,
    inset: 0,
    background: 'rgba(0, 0, 0, 0.7)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 10000,
    backdropFilter: 'blur(4px)',
  },
  modal: {
    background: 'var(--bg-primary)',
    border: '1px solid var(--border-color)',
    borderRadius: 12,
    width: '90%',
    maxWidth: 640,
    maxHeight: '85vh',
    overflow: 'auto',
    boxShadow: '0 20px 60px rgba(0, 0, 0, 0.5)',
  },
  header: {
    padding: '32px 32px 0',
    textAlign: 'center' as const,
  },
  title: {
    fontSize: 24,
    fontWeight: 700,
    color: 'var(--text-primary)',
    marginBottom: 8,
  },
  subtitle: {
    fontSize: 14,
    color: 'var(--text-secondary)',
    lineHeight: 1.6,
    marginBottom: 24,
  },
  body: {
    padding: '0 32px 32px',
  },
  stepIndicator: {
    display: 'flex',
    justifyContent: 'center',
    gap: 8,
    marginBottom: 24,
  },
  dot: {
    width: 8,
    height: 8,
    borderRadius: '50%',
    transition: 'background 0.2s',
  },
  footer: {
    display: 'flex',
    justifyContent: 'flex-end',
    gap: 12,
    padding: '16px 32px',
    borderTop: '1px solid var(--border-color)',
  },
  btn: {
    padding: '8px 20px',
    borderRadius: 6,
    border: 'none',
    fontSize: 13,
    fontWeight: 500,
    cursor: 'pointer',
    transition: 'opacity 0.15s',
  },
  btnPrimary: {
    background: 'var(--accent)',
    color: '#fff',
  },
  btnSecondary: {
    background: 'var(--bg-tertiary)',
    color: 'var(--text-primary)',
    border: '1px solid var(--border-color)',
  },
};

export default function SetupWizard({ onComplete }: SetupWizardProps) {
  const [step, setStep] = useState(0);
  const totalSteps = 3;

  return (
    <div style={styles.overlay}>
      <div style={styles.modal}>
        {/* Step Indicator */}
        <div style={{ ...styles.header }}>
          <div style={styles.stepIndicator}>
            {Array.from({ length: totalSteps }).map((_, i) => (
              <div
                key={i}
                style={{
                  ...styles.dot,
                  background: i <= step ? 'var(--accent)' : 'var(--bg-tertiary)',
                }}
              />
            ))}
          </div>
        </div>

        <div style={styles.body}>
          {step === 0 && (
            <div style={{ textAlign: 'center' }}>
              <div style={{ fontSize: 48, marginBottom: 16 }}>
                <svg width="64" height="64" viewBox="0 0 16 16" fill="var(--accent)">
                  <path d="M14.85 3H1.15L1 3.15v9.7l.15.15h13.7l.15-.15V3.15L14.85 3zM14 12H2V4h12v8z" />
                  <path d="M4 6l2 2-2 2 1 1 3-3-3-3-1 1zm4 4h4v1H8v-1z" />
                </svg>
              </div>
              <h2 style={styles.title}>Welcome to LocalCode</h2>
              <p style={styles.subtitle}>
                A privacy-first AI code editor that runs locally on your machine.
                Let's get you set up with an LLM provider so you can start coding with AI assistance.
              </p>
            </div>
          )}

          {step === 1 && (
            <div>
              <h2 style={{ ...styles.title, textAlign: 'center', fontSize: 20 }}>
                Configure LLM Provider
              </h2>
              <p style={{ ...styles.subtitle, textAlign: 'center' }}>
                Choose a local model to run on your machine, or connect a cloud provider.
                You can always change this later in Settings.
              </p>
              <div style={{ maxHeight: '50vh', overflow: 'auto' }}>
                <LLMSettings />
              </div>
            </div>
          )}

          {step === 2 && (
            <div style={{ textAlign: 'center' }}>
              <div style={{ fontSize: 48, marginBottom: 16 }}>
                <svg width="64" height="64" viewBox="0 0 16 16" fill="var(--accent)">
                  <path d="M8 1a7 7 0 100 14A7 7 0 008 1zm3.35 5.35l-4 4a.5.5 0 01-.7 0l-2-2a.5.5 0 01.7-.7L7 9.29l3.65-3.64a.5.5 0 01.7.7z" />
                </svg>
              </div>
              <h2 style={styles.title}>You're All Set!</h2>
              <p style={styles.subtitle}>
                You can start using LocalCode. Toggle Agent Mode in the AI Chat panel
                to let the AI edit files, run commands, and help you build.
              </p>
            </div>
          )}
        </div>

        <div style={styles.footer}>
          {step > 0 && (
            <button
              style={{ ...styles.btn, ...styles.btnSecondary }}
              onClick={() => setStep(step - 1)}
            >
              Back
            </button>
          )}
          {step === 0 && (
            <button
              style={{ ...styles.btn, ...styles.btnSecondary }}
              onClick={onComplete}
            >
              Skip Setup
            </button>
          )}
          <button
            style={{ ...styles.btn, ...styles.btnPrimary }}
            onClick={() => {
              if (step < totalSteps - 1) {
                setStep(step + 1);
              } else {
                onComplete();
              }
            }}
          >
            {step === totalSteps - 1 ? 'Get Started' : 'Next'}
          </button>
        </div>
      </div>
    </div>
  );
}
