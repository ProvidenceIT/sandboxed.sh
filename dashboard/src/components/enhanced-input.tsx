'use client';

import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { listLibraryCommands, getVisibleAgents, type CommandSummary } from '@/lib/api';
import { cn } from '@/lib/utils';

// Built-in oh-my-opencode commands
const BUILTIN_COMMANDS: CommandSummary[] = [
  { name: 'ralph-loop', description: 'Start self-referential development loop until completion', path: 'builtin' },
  { name: 'cancel-ralph', description: 'Cancel active Ralph Loop', path: 'builtin' },
  { name: 'start-work', description: 'Start Sisyphus work session from Prometheus plan', path: 'builtin' },
  { name: 'refactor', description: 'Intelligent refactoring with LSP, AST-grep, and TDD verification', path: 'builtin' },
  { name: 'init-deep', description: 'Initialize hierarchical AGENTS.md knowledge base', path: 'builtin' },
];

export interface SubmitPayload {
  content: string;
  agent?: string;
}

interface EnhancedInputProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: (payload: SubmitPayload) => void;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
}

interface AutocompleteItem {
  type: 'command' | 'agent';
  name: string;
  description: string | null;
  source?: string;
}

export function EnhancedInput({
  value,
  onChange,
  onSubmit,
  placeholder = "Message the root agent...",
  disabled = false,
  className,
}: EnhancedInputProps) {
  const [commands, setCommands] = useState<CommandSummary[]>([]);
  const [agents, setAgents] = useState<string[]>([]);
  const [showAutocomplete, setShowAutocomplete] = useState(false);
  const [autocompleteItems, setAutocompleteItems] = useState<AutocompleteItem[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [autocompleteType, setAutocompleteType] = useState<'command' | 'agent' | null>(null);
  const [triggerPosition, setTriggerPosition] = useState(0);

  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const autocompleteRef = useRef<HTMLDivElement>(null);

  // Load commands and agents on mount
  useEffect(() => {
    async function loadData() {
      try {
        // Load library commands
        const libraryCommands = await listLibraryCommands();
        setCommands([...BUILTIN_COMMANDS, ...libraryCommands]);
      } catch {
        setCommands(BUILTIN_COMMANDS);
      }

      try {
        // Load agents
        const agentsData = await getVisibleAgents();
        const agentNames = parseAgentNames(agentsData);
        setAgents(agentNames);
      } catch {
        setAgents(['Sisyphus', 'oracle', 'explore', 'librarian', 'plan']);
      }
    }
    loadData();
  }, []);

  const parseAgentNames = (payload: unknown): string[] => {
    const normalizeEntry = (entry: unknown): string | null => {
      if (typeof entry === 'string') return entry;
      if (entry && typeof entry === 'object') {
        const name = (entry as { name?: unknown }).name;
        if (typeof name === 'string') return name;
        const id = (entry as { id?: unknown }).id;
        if (typeof id === 'string') return id;
      }
      return null;
    };

    const raw = Array.isArray(payload)
      ? payload
      : (payload as { agents?: unknown })?.agents;
    if (!Array.isArray(raw)) return [];

    const names = raw
      .map(normalizeEntry)
      .filter((name): name is string => Boolean(name));
    return Array.from(new Set(names));
  };

  // Auto-resize textarea
  const adjustTextareaHeight = useCallback(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;

    textarea.style.height = "auto";
    const lineHeight = 20;
    const maxLines = 10;
    const maxHeight = lineHeight * maxLines;
    const newHeight = Math.min(textarea.scrollHeight, maxHeight);
    textarea.style.height = `${newHeight}px`;
  }, []);

  useEffect(() => {
    adjustTextareaHeight();
  }, [value, adjustTextareaHeight]);

  // Detect triggers (/ or @) and update autocomplete
  useEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;

    const cursorPos = textarea.selectionStart;
    const textBeforeCursor = value.substring(0, cursorPos);

    // Check for / command trigger at start of line or after whitespace
    const commandMatch = textBeforeCursor.match(/(?:^|\s)(\/[\w-]*)$/);
    if (commandMatch) {
      const searchTerm = commandMatch[1].substring(1).toLowerCase();
      const filtered = commands.filter(cmd =>
        cmd.name.toLowerCase().includes(searchTerm)
      );
      setAutocompleteItems(filtered.map(cmd => ({
        type: 'command',
        name: cmd.name,
        description: cmd.description,
        source: cmd.path === 'builtin' ? 'oh-my-opencode' : 'library',
      })));
      setAutocompleteType('command');
      setTriggerPosition(cursorPos - commandMatch[1].length);
      setShowAutocomplete(filtered.length > 0);
      setSelectedIndex(0);
      return;
    }

    // Check for @ agent trigger at start of message
    const agentMatch = textBeforeCursor.match(/^@([\w-]*)$/);
    if (agentMatch) {
      const searchTerm = agentMatch[1].toLowerCase();
      const filtered = agents.filter(agent =>
        agent.toLowerCase().includes(searchTerm)
      );
      setAutocompleteItems(filtered.map(agent => ({
        type: 'agent',
        name: agent,
        description: getAgentDescription(agent),
      })));
      setAutocompleteType('agent');
      setTriggerPosition(0);
      setShowAutocomplete(filtered.length > 0);
      setSelectedIndex(0);
      return;
    }

    setShowAutocomplete(false);
    setAutocompleteType(null);
  }, [value, commands, agents]);

  const getAgentDescription = (name: string): string => {
    const descriptions: Record<string, string> = {
      'Sisyphus': 'Main orchestrator with parallel execution',
      'oracle': 'Architecture, code review, strategy (GPT)',
      'explore': 'Fast codebase exploration and search',
      'librarian': 'Documentation lookup and research',
      'plan': 'Prometheus planner for structured work',
      'frontend-ui-ux-engineer': 'UI/UX development specialist',
      'document-writer': 'Technical documentation expert',
      'multimodal-looker': 'Visual content analysis',
    };
    return descriptions[name] || 'Specialized agent';
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (showAutocomplete) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex(prev =>
          prev < autocompleteItems.length - 1 ? prev + 1 : 0
        );
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex(prev =>
          prev > 0 ? prev - 1 : autocompleteItems.length - 1
        );
        return;
      }
      if (e.key === 'Tab' || e.key === 'Enter') {
        if (autocompleteItems.length > 0) {
          e.preventDefault();
          selectItem(autocompleteItems[selectedIndex]);
          return;
        }
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        setShowAutocomplete(false);
        return;
      }
    }

    // Normal Enter to submit (without Shift)
    if (e.key === 'Enter' && !e.shiftKey && !showAutocomplete) {
      e.preventDefault();
      handleSubmit();
    }
  };

  const selectItem = (item: AutocompleteItem) => {
    if (item.type === 'command') {
      // Replace the /partial with /command
      const before = value.substring(0, triggerPosition);
      const after = value.substring(textareaRef.current?.selectionStart || value.length);
      const newValue = `${before}/${item.name} ${after}`.trim();
      onChange(newValue);
    } else if (item.type === 'agent') {
      // Replace @partial with @Agent badge, add space
      const newValue = `@${item.name} `;
      onChange(newValue);
    }
    setShowAutocomplete(false);
    textareaRef.current?.focus();
  };

  const handleSubmit = () => {
    if (!value.trim() || disabled) return;

    // Parse @agent mention from the beginning of the message
    const agentMatch = value.match(/^@([\w-]+)\s*/);
    if (agentMatch) {
      const agent = agentMatch[1];
      const content = value.substring(agentMatch[0].length).trim();
      // If there's actual content after the mention, send with agent
      // If only the mention, send just the mention as content (edge case)
      if (content) {
        onSubmit({ content, agent });
      } else {
        onSubmit({ content: value });
      }
    } else {
      onSubmit({ content: value });
    }
  };

  // Render the value with @Agent badges
  const renderValue = useMemo(() => {
    const agentMatch = value.match(/^(@[\w-]+)(\s.*)?$/s);
    if (agentMatch) {
      return {
        hasBadge: true,
        badge: agentMatch[1],
        rest: agentMatch[2] || '',
      };
    }
    return { hasBadge: false, badge: '', rest: value };
  }, [value]);

  return (
    <div className="relative flex-1">
      {/* Overlay for badge rendering */}
      {renderValue.hasBadge && (
        <div
          className="absolute left-4 top-3 pointer-events-none z-10 flex items-center gap-1"
          aria-hidden="true"
        >
          <span className="inline-flex items-center rounded bg-pink-500/30 text-pink-300 px-1.5 py-0.5 text-sm font-medium border border-pink-500/30">
            {renderValue.badge}
          </span>
        </div>
      )}

      <textarea
        ref={textareaRef}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={placeholder}
        disabled={disabled}
        rows={1}
        className={cn(
          "w-full rounded-xl border border-white/[0.06] bg-white/[0.02] px-4 py-3 text-sm text-white placeholder-white/30 focus:border-indigo-500/50 focus:outline-none transition-[border-color,height] duration-150 ease-out resize-none overflow-y-auto leading-5",
          renderValue.hasBadge && "text-transparent caret-white",
          className
        )}
        style={{
          minHeight: "46px",
          // Make text transparent where badge is, but keep caret visible
          ...(renderValue.hasBadge ? {
            background: 'linear-gradient(to right, transparent 0px, transparent ' + (renderValue.badge.length * 8 + 30) + 'px, transparent ' + (renderValue.badge.length * 8 + 30) + 'px)',
          } : {}),
        }}
      />

      {/* Text overlay for non-badge part when badge is present */}
      {renderValue.hasBadge && (
        <div
          className="absolute left-4 top-3 pointer-events-none z-10 flex items-center text-sm text-white"
          style={{ paddingLeft: `${renderValue.badge.length * 8 + 12}px` }}
          aria-hidden="true"
        >
          {renderValue.rest}
        </div>
      )}

      {/* Autocomplete dropdown */}
      {showAutocomplete && autocompleteItems.length > 0 && (
        <div
          ref={autocompleteRef}
          className="absolute bottom-full left-0 right-0 mb-2 max-h-64 overflow-y-auto rounded-lg border border-white/[0.08] bg-[#1a1a1a] shadow-xl z-50"
        >
          {autocompleteItems.map((item, index) => (
            <button
              key={`${item.type}-${item.name}`}
              type="button"
              onClick={() => selectItem(item)}
              className={cn(
                "w-full px-3 py-2.5 text-left flex items-start gap-3 transition-colors",
                index === selectedIndex
                  ? "bg-white/[0.08]"
                  : "hover:bg-white/[0.04]"
              )}
            >
              <span className="text-white/40 font-mono text-sm shrink-0">
                {item.type === 'command' ? '/' : '@'}
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-white text-sm">
                    {item.name}
                  </span>
                  {item.source && (
                    <span className="text-xs text-white/30 px-1.5 py-0.5 rounded bg-white/[0.05]">
                      {item.source}
                    </span>
                  )}
                </div>
                {item.description && (
                  <p className="text-xs text-white/50 mt-0.5 truncate">
                    {item.description}
                  </p>
                )}
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
