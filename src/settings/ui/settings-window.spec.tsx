import { act, fireEvent, render, screen } from '@testing-library/react';
import { userEvent } from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import type { SaveResult } from '../application/ports.ts';
import { SettingsStore } from '../application/settings-store.ts';
import type { Connection } from '../domain/connection.ts';
import { PORT_MESSAGE, SAME_SHORTCUT_MESSAGE, type Settings } from '../domain/settings.ts';
import {
  FAKE_DEFAULTS,
  FakeSettingsGateway,
  LANGUAGE_MESSAGE,
} from '../infrastructure/fake-settings-gateway.ts';
import { SettingsWindow } from './settings-window.tsx';

function rejected<T>(reason: unknown): Promise<T> {
  const { promise, reject } = Promise.withResolvers<T>();
  reject(reason);
  return promise;
}

class ControlledGateway extends FakeSettingsGateway {
  readonly calls: string[] = [];
  loadGate: Promise<void> = Promise.resolve();
  loadFailure: unknown = undefined;
  saveGate: Promise<void> = Promise.resolve();
  checkGate: Promise<void> = Promise.resolve();
  statusFailure: unknown = undefined;
  suspendFailure: unknown = undefined;

  override async load(): Promise<Settings> {
    await this.loadGate;
    return this.loadFailure === undefined ? super.load() : rejected(this.loadFailure);
  }

  override async save(settings: Settings): Promise<SaveResult> {
    await this.saveGate;
    return super.save(settings);
  }

  override async connectionStatus(): Promise<Connection> {
    return this.statusFailure === undefined
      ? super.connectionStatus()
      : new Promise<Connection>(() => undefined);
  }

  override async checkConnection(): Promise<Connection> {
    const result = super.checkConnection();
    await this.checkGate;
    return result;
  }

  override suspendShortcuts(): Promise<void> {
    this.calls.push('suspend');
    return this.suspendFailure === undefined ? Promise.resolve() : rejected(this.suspendFailure);
  }

  override resumeShortcuts(): Promise<void> {
    this.calls.push('resume');
    return Promise.resolve();
  }
}

async function show(gateway = new ControlledGateway()): Promise<ControlledGateway> {
  const store = new SettingsStore(gateway);
  store.load();
  render(<SettingsWindow store={store} />);
  await act(() => Promise.resolve());
  return gateway;
}

function changeButton(label: string): HTMLElement {
  return screen.getByRole('button', { name: `Change ${label}` });
}

function chordOf(field: string): string | null {
  return screen.getByTestId(`chord-${field}`).textContent;
}

function focused(element: HTMLElement): boolean {
  return element.matches(':focus');
}

function saveButton(): HTMLElement {
  return screen.getByRole('button', { name: 'Save' });
}

function field(name: string): HTMLElement {
  return screen.getByRole('textbox', { name });
}

describe('SettingsWindow loading', () => {
  it('says it is loading until the settings arrive', async () => {
    const gateway = new ControlledGateway();
    const gate = Promise.withResolvers<undefined>();
    gateway.loadGate = gate.promise;
    await show(gateway);
    expect(screen.getByRole('heading', { name: 'Settings' })).toBeDefined();
    expect(screen.getByText('Loading settings…')).toBeDefined();
    await act(async () => {
      gate.resolve(undefined);
      await gate.promise;
    });
    expect(screen.queryByText('Loading settings…')).toBeNull();
  });

  it('explains a failed load', async () => {
    const gateway = new ControlledGateway();
    gateway.loadFailure = new Error('No core.');
    await show(gateway);
    expect(screen.getByRole('alert').textContent).toBe('Settings could not be loaded. No core.');
  });

  it('shows every setting', async () => {
    await show();
    expect(chordOf('dictateShortcut')).toBe('F9');
    expect(chordOf('readAloudShortcut')).toBe('Ctrl+Alt+R');
    expect(
      ['Host', 'Speech recognition port', 'Voice port', 'Dictation language'].map((name) =>
        field(name).getAttribute('value'),
      ),
    ).toEqual(['127.0.0.1', '10300', '10200', 'auto']);
    expect(screen.getByRole('switch', { name: 'Show the listening indicator' })).toHaveProperty(
      'checked',
      true,
    );
    expect(screen.getByRole('switch', { name: 'Reduce motion' })).toHaveProperty('checked', false);
    expect(
      screen.getByRole('switch', { name: 'Start at sign-in (hidden in the tray)' }),
    ).toHaveProperty('checked', false);
  });

  it('names the sections', async () => {
    await show();
    expect(
      ['Shortcuts', 'Voice server', 'Indicator', 'Startup'].map(
        (name) => screen.getByRole('group', { name }).tagName,
      ),
    ).toEqual(['FIELDSET', 'FIELDSET', 'FIELDSET', 'FIELDSET']);
  });
});

describe('SettingsWindow shortcuts', () => {
  it('records a chord after modifier-only presses and keeps focus on the button', async () => {
    const gateway = await show();
    await userEvent.click(changeButton('Dictate (hold to talk)'));
    const recorder = screen.getByRole('button', { name: 'Press a shortcut, or Esc to cancel' });
    expect(focused(recorder)).toBe(true);
    await userEvent.keyboard('[ControlLeft]');
    expect(screen.getByRole('button', { name: 'Press a shortcut, or Esc to cancel' })).toBe(
      recorder,
    );
    await userEvent.keyboard('[ControlLeft>][ShiftLeft>][Space][/ShiftLeft][/ControlLeft]');
    expect(chordOf('dictateShortcut')).toBe('Ctrl+Shift+Space');
    expect(focused(changeButton('Dictate (hold to talk)'))).toBe(true);
    expect(gateway.calls).toEqual(['suspend', 'resume']);
  });

  it('focuses the button it records on, even where clicks do not focus buttons', async () => {
    await show();
    fireEvent.click(changeButton('Dictate (hold to talk)'));
    expect(
      focused(screen.getByRole('button', { name: 'Press a shortcut, or Esc to cancel' })),
    ).toBe(true);
  });

  it('starts recording from the keyboard', async () => {
    await show();
    changeButton('Read selection aloud').focus();
    await userEvent.keyboard('[Enter]');
    await userEvent.keyboard('[F8]');
    expect(chordOf('readAloudShortcut')).toBe('F8');
  });

  it('cancels recording with Escape', async () => {
    const gateway = await show();
    await userEvent.click(changeButton('Read selection aloud'));
    await userEvent.keyboard('[Escape]');
    expect(chordOf('readAloudShortcut')).toBe('Ctrl+Alt+R');
    expect(gateway.calls).toEqual(['suspend', 'resume']);
  });

  it('cancels recording when focus leaves', async () => {
    const gateway = await show();
    await userEvent.click(changeButton('Read selection aloud'));
    await userEvent.click(field('Host'));
    expect(changeButton('Read selection aloud').textContent).toBe('Change');
    expect(gateway.calls).toEqual(['suspend', 'resume']);
  });

  it('refuses the same shortcut twice', async () => {
    await show();
    await userEvent.click(changeButton('Dictate (hold to talk)'));
    await userEvent.keyboard('[ControlLeft>][AltLeft>][KeyR][/AltLeft][/ControlLeft]');
    const errors = screen.getAllByText(SAME_SHORTCUT_MESSAGE);
    expect(errors.map((error) => error.id)).toEqual([
      'dictateShortcut-error',
      'readAloudShortcut-error',
    ]);
    expect(changeButton('Dictate (hold to talk)').getAttribute('aria-describedby')).toBe(
      'dictateShortcut-error',
    );
    expect(saveButton()).toHaveProperty('disabled', true);
  });

  it('resets a shortcut to its default', async () => {
    await show();
    await userEvent.click(changeButton('Dictate (hold to talk)'));
    await userEvent.keyboard('[F8]');
    await userEvent.click(screen.getByRole('button', { name: 'Reset Dictate (hold to talk)' }));
    expect(chordOf('dictateShortcut')).toBe('F9');
  });

  it('shows a failure to suspend the global shortcuts', async () => {
    const gateway = new ControlledGateway();
    gateway.suspendFailure = new Error('No hotkeys.');
    await show(gateway);
    await userEvent.click(changeButton('Dictate (hold to talk)'));
    expect(screen.getAllByRole('status')[1]?.textContent).toBe('No hotkeys.');
  });
});

describe('SettingsWindow fields', () => {
  it('marks a field invalid only while its value is wrong', async () => {
    await show();
    const port = field('Speech recognition port');
    expect(port.getAttribute('aria-invalid')).toBe('false');
    expect(port.getAttribute('aria-describedby')).toBeNull();
    await userEvent.clear(port);
    await userEvent.type(port, '70000');
    expect(port.getAttribute('aria-invalid')).toBe('true');
    expect(port.getAttribute('aria-describedby')).toBe('sttPort-error');
    expect(screen.getByText(PORT_MESSAGE).id).toBe('sttPort-error');
    expect(saveButton()).toHaveProperty('disabled', true);
  });

  it('enables Save and Revert only when the draft changed', async () => {
    await show();
    const revert = screen.getByRole('button', { name: 'Revert' });
    expect(saveButton()).toHaveProperty('disabled', true);
    expect(revert).toHaveProperty('disabled', true);
    await userEvent.click(screen.getByRole('switch', { name: 'Reduce motion' }));
    expect(saveButton()).toHaveProperty('disabled', false);
    expect(revert).toHaveProperty('disabled', false);
    await userEvent.click(revert);
    expect(screen.getByRole('switch', { name: 'Reduce motion' })).toHaveProperty('checked', false);
  });

  it('saves, shows the stored values, and says Saved', async () => {
    const gateway = new ControlledGateway();
    const gate = Promise.withResolvers<undefined>();
    gateway.saveGate = gate.promise;
    await show(gateway);
    expect(screen.getAllByRole('status')[1]?.textContent).toBe('');
    await userEvent.clear(field('Host'));
    await userEvent.type(field('Host'), 'Voice.Local');
    await userEvent.click(saveButton());
    expect(saveButton()).toHaveProperty('disabled', true);
    await act(async () => {
      gate.resolve(undefined);
      await gate.promise;
    });
    expect(field('Host').getAttribute('value')).toBe('voice.local');
    expect(screen.getAllByRole('status')[1]?.textContent).toBe('Saved.');
  });

  it('saves with Enter and shows the core field error inline', async () => {
    await show();
    await userEvent.clear(field('Dictation language'));
    await userEvent.type(field('Dictation language'), 'English[Enter]');
    expect(screen.getByText(LANGUAGE_MESSAGE).id).toBe('sttLanguage-error');
    expect(field('Dictation language').getAttribute('aria-invalid')).toBe('true');
    expect(screen.getAllByRole('status')[1]?.textContent).toBe('');
  });

  it('keeps the window in place when the form is submitted', async () => {
    await show();
    const submitted: Event[] = [];
    document.addEventListener('submit', (event) => submitted.push(event));
    await userEvent.type(field('Host'), 'x[Enter]');
    expect(submitted.map((event) => event.defaultPrevented)).toEqual([true]);
  });

  it('does not spell-check addresses and codes', async () => {
    await show();
    expect(
      ['Host', 'Speech recognition port', 'Voice port', 'Dictation language'].map((name) =>
        field(name).getAttribute('spellcheck'),
      ),
    ).toEqual(['false', 'false', 'false', 'false']);
  });

  it('turns switches on and off', async () => {
    await show();
    await userEvent.click(
      screen.getByRole('switch', { name: 'Start at sign-in (hidden in the tray)' }),
    );
    expect(
      screen.getByRole('switch', { name: 'Start at sign-in (hidden in the tray)' }),
    ).toHaveProperty('checked', true);
    expect(FAKE_DEFAULTS.startAtSignIn).toBe(false);
  });
});

describe('SettingsWindow connection', () => {
  it('shows the connection and tests it on request', async () => {
    const gateway = new ControlledGateway();
    const gate = Promise.withResolvers<undefined>();
    gateway.checkGate = gate.promise;
    await show(gateway);
    const status = screen.getAllByRole('status')[0];
    expect(status?.textContent).toBe('Speech recognition and voice are reachable at 127.0.0.1.');
    await userEvent.click(screen.getByRole('button', { name: 'Test connection' }));
    expect(screen.getByRole('button', { name: 'Testing…' })).toHaveProperty('disabled', true);
    expect(status?.textContent).toBe('Checking the voice server.');
    await act(async () => {
      gate.resolve(undefined);
      await gate.promise;
    });
    expect(screen.getByRole('button', { name: 'Test connection' })).toHaveProperty(
      'disabled',
      false,
    );
  });

  it('says it is checking before the first answer', async () => {
    const gateway = new ControlledGateway();
    gateway.statusFailure = 'never answers';
    await show(gateway);
    expect(screen.getAllByRole('status')[0]?.textContent).toBe('Checking the voice server…');
  });
});
