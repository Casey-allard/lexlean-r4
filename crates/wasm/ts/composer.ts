/**
 * UOR-R4 Agentic Chat Composer (TypeScript Source of Truth)
 *
 * Provides the strongly-typed state machine backing the Claude Cowork / Cursor-style
 * rich chatbox (`#claude-chat-form` / `#claude-chat-input`):
 * - `/slash` command autocomplete palette (`/think`, `/teach`, `/patch`, `/arxiv`, `/consolidate`, `/otel`, `/math`, `/clear`)
 * - `@context` mention chips (`@file`, `@arxiv`, `@claim`) attached to the prompt envelope
 * - Execution mode selector (`agentic`, `thinking`, `teach`, `math`)
 * - Terminal-style Up/Down prompt history stack and live BPE token estimation
 *
 * Compiled into `public/app.js` to preserve the strict 6-file `prismpm/browser-provenance/1` closure.
 */

export type ComposerMode = 'agentic' | 'thinking' | 'teach' | 'math';

export interface ComposerAttachmentChip {
  readonly id: string;
  readonly kind: 'file' | 'arxiv' | 'claim' | 'mode';
  readonly label: string;
  readonly payload: string;
}

export interface SlashCommandSpec {
  readonly command: string;
  readonly title: string;
  readonly badge: string;
  readonly description: string;
  readonly template: string;
  readonly mode?: ComposerMode;
  readonly action?: 'consolidate' | 'otel' | 'clear';
}

export interface MentionCandidate {
  readonly id: string;
  readonly kind: 'file' | 'arxiv' | 'claim';
  readonly label: string;
  readonly detail: string;
  readonly payload: string;
}

export interface ComposerState {
  rawText: string;
  mode: ComposerMode;
  attachments: ComposerAttachmentChip[];
  history: string[];
  historyIndex: number;
  autocompleteOpen: boolean;
  autocompleteTrigger: 'slash' | 'mention' | null;
  autocompleteQuery: string;
  autocompleteSelectedIndex: number;
}

export const AGENTIC_SLASH_COMMANDS: readonly SlashCommandSpec[] = [
  {
    command: '/think',
    title: 'Deep Geometric <thinking> Trace',
    badge: '256D Q12',
    description: 'Force multi-step Cl(0,6) + 256D Q12 latent reasoning before answering',
    template: 'Analyze step-by-step with <thinking> geometric coordinates: ',
    mode: 'thinking',
  },
  {
    command: '/teach',
    title: 'Teach & Admit Online LUT Row',
    badge: 'LUT /7+/8',
    description: 'Insert a Teach: prompt => <thinking>...</thinking>reply rule into weights',
    template: 'Teach: ',
    mode: 'teach',
  },
  {
    command: '/patch',
    title: 'Synthesize Workspace Code Patch',
    badge: 'IDE Diff',
    description: 'Generate a unified diff for the pinned workspace file',
    template: 'Write a patch for the active workspace file to implement: ',
    mode: 'agentic',
  },
  {
    command: '/arxiv',
    title: 'Query ArXiv Frontier Literature',
    badge: '10 Papers',
    description: 'Test or admit mathematical hypotheses from mined arXiv papers',
    template: 'Explain how DeepSeek-V2 Multi-Head Latent Attention (arXiv:2405.04434) maps to 256D Q12 context carry-forward',
    mode: 'thinking',
  },
  {
    command: '/math',
    title: 'Exact M6 Geometric Arithmetic',
    badge: 'R4-27',
    description: 'Run exact radix-B orbit-digit addition & PRLL likelihood conservation',
    template: 'What is 144 + 233?',
    mode: 'math',
  },
  {
    command: '/consolidate',
    title: 'Run Web Worker Weight Consolidation',
    badge: 'Worker Q12',
    description: 'Open Consolidation drawer and optimize UORLUT01/8 weights in background',
    template: '',
    action: 'consolidate',
  },
  {
    command: '/otel',
    title: 'Inspect 5-Tier OTel GLLM Waterfall',
    badge: 'u16 Dedup',
    description: 'Open OpenTelemetry uor.gllm.trace/1 waterfall & SpanLinks drawer',
    template: '',
    action: 'otel',
  },
  {
    command: '/clear',
    title: 'Reset Thread & Rolling Context',
    badge: 'Reset',
    description: 'Clear current conversation thread and reset 256D Q12 RollingContextState',
    template: '',
    action: 'clear',
  },
];

export class AgenticTypeScriptComposer {
  private state: ComposerState;

  constructor() {
    this.state = {
      rawText: '',
      mode: 'agentic',
      attachments: [],
      history: [],
      historyIndex: -1,
      autocompleteOpen: false,
      autocompleteTrigger: null,
      autocompleteQuery: '',
      autocompleteSelectedIndex: 0,
    };
  }

  public getState(): ComposerState {
    return this.state;
  }

  public setMode(mode: ComposerMode): void {
    this.state.mode = mode;
  }

  public addAttachment(chip: ComposerAttachmentChip): boolean {
    if (this.state.attachments.some((existing) => existing.id === chip.id)) {
      return false;
    }
    this.state.attachments.push(chip);
    return true;
  }

  public removeAttachment(id: string): void {
    this.state.attachments = this.state.attachments.filter((c) => c.id !== id);
  }

  public clearAttachments(): void {
    this.state.attachments = [];
  }

  public estimateTokenCount(text: string): number {
    const trimmed = text.trim();
    if (!trimmed) return this.state.attachments.length * 6;
    const words = trimmed.split(/\s+/).length;
    const punctuation = (trimmed.match(/[.,!?;:()[\]{}"'`/\\<>+=*-]/g) || []).length;
    return Math.max(1, words + Math.ceil(punctuation * 0.5) + this.state.attachments.length * 6);
  }

  public detectAutocompleteTrigger(text: string, cursorOffset: number): {
    trigger: 'slash' | 'mention' | null;
    query: string;
    tokenStart: number;
  } {
    const prefix = text.slice(0, Math.max(0, cursorOffset));
    const slashMatch = prefix.match(/(?:^|\s)(\/[a-zA-Z0-9_-]*)$/);
    if (slashMatch) {
      const token = slashMatch[1];
      return {
        trigger: 'slash',
        query: token.slice(1).toLowerCase(),
        tokenStart: prefix.length - token.length,
      };
    }
    const mentionMatch = prefix.match(/(?:^|\s)(@[a-zA-Z0-9_.:/-]*)$/);
    if (mentionMatch) {
      const token = mentionMatch[1];
      return {
        trigger: 'mention',
        query: token.slice(1).toLowerCase(),
        tokenStart: prefix.length - token.length,
      };
    }
    return { trigger: null, query: '', tokenStart: -1 };
  }

  public filterSlashCommands(query: string): SlashCommandSpec[] {
    const q = query.trim().toLowerCase();
    if (!q) return [...AGENTIC_SLASH_COMMANDS];
    return AGENTIC_SLASH_COMMANDS.filter(
      (cmd) =>
        cmd.command.slice(1).toLowerCase().includes(q) ||
        cmd.title.toLowerCase().includes(q) ||
        cmd.badge.toLowerCase().includes(q)
    );
  }

  public pushHistory(prompt: string): void {
    const clean = prompt.trim();
    if (!clean) return;
    if (this.state.history.length === 0 || this.state.history[0] !== clean) {
      this.state.history.unshift(clean);
      if (this.state.history.length > 32) {
        this.state.history.pop();
      }
    }
    this.state.historyIndex = -1;
  }

  public stepHistory(direction: 'up' | 'down'): string | null {
    if (this.state.history.length === 0) return null;
    if (direction === 'up') {
      const nextIdx = Math.min(this.state.historyIndex + 1, this.state.history.length - 1);
      this.state.historyIndex = nextIdx;
      return this.state.history[nextIdx];
    }
    if (this.state.historyIndex <= 0) {
      this.state.historyIndex = -1;
      return '';
    }
    this.state.historyIndex -= 1;
    return this.state.history[this.state.historyIndex];
  }

  public getAttachmentBadges(): string[] {
    return this.state.attachments.map((chip) => `@${chip.label}`);
  }

  public synthesizePromptEnvelope(rawInput: string): string {
    const body = rawInput.trim();
    if (!body) return '';

    if (this.state.mode === 'teach' && !/^teach\s*:/i.test(body) && body.includes('=>')) {
      return `Teach: ${body}`;
    }

    return body;
  }
}
