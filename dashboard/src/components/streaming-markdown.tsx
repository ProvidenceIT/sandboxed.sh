"use client";

import { memo, useMemo, useRef, useEffect, useState } from "react";
import { MarkdownContent } from "./markdown-content";
import { cn } from "@/lib/utils";

interface StreamingMarkdownProps {
  content: string;
  isStreaming: boolean;
  className?: string;
  basePath?: string;
  /** Time in ms to wait before considering a block "stable" */
  stabilizeDelay?: number;
}

/**
 * Efficient markdown rendering for streaming content.
 *
 * Strategy:
 * 1. Split content into blocks (paragraphs separated by double newlines)
 * 2. Render completed blocks as cached markdown
 * 3. Render the last (actively streaming) block as plain text
 * 4. Convert to markdown once the block stabilizes (no updates for stabilizeDelay ms)
 *
 * This reduces DOM mutations from O(content.length) to O(last_block.length)
 */
export const StreamingMarkdown = memo(function StreamingMarkdown({
  content,
  isStreaming,
  className,
  basePath,
  stabilizeDelay = 300,
}: StreamingMarkdownProps) {
  // Split content into blocks (paragraphs)
  const blocks = useMemo(() => {
    if (!content) return [];
    // Split by double newlines, but keep code blocks intact
    const parts = content.split(/\n\n+/);
    return parts.filter(p => p.trim());
  }, [content]);

  // Track which blocks have been "seen" as stable
  const stableBlocksRef = useRef<Map<string, string>>(new Map());

  // Track when the last block was updated for stabilization
  const lastUpdateRef = useRef<number>(Date.now());
  const [lastBlockStable, setLastBlockStable] = useState(false);

  // Get stable blocks (all except the last one during streaming)
  const stableBlocks = useMemo(() => {
    if (!isStreaming || blocks.length <= 1) {
      return blocks;
    }
    return blocks.slice(0, -1);
  }, [blocks, isStreaming]);

  // Get the streaming block (last block during streaming)
  const streamingBlock = useMemo(() => {
    if (!isStreaming || blocks.length === 0) {
      return null;
    }
    return blocks[blocks.length - 1];
  }, [blocks, isStreaming]);

  // Stabilization timer for the last block
  useEffect(() => {
    if (!isStreaming || !streamingBlock) {
      setLastBlockStable(false);
      return;
    }

    lastUpdateRef.current = Date.now();
    setLastBlockStable(false);

    const timer = setTimeout(() => {
      setLastBlockStable(true);
    }, stabilizeDelay);

    return () => clearTimeout(timer);
  }, [streamingBlock, isStreaming, stabilizeDelay]);

  // When not streaming, render everything as markdown
  if (!isStreaming) {
    return (
      <MarkdownContent
        content={content}
        className={className}
        basePath={basePath}
      />
    );
  }

  // During streaming: render stable blocks as markdown, streaming block as text
  return (
    <div className={cn("streaming-markdown", className)}>
      {/* Render stable blocks as cached markdown */}
      {stableBlocks.map((block, index) => (
        <MemoizedBlock
          key={`stable-${index}-${block.slice(0, 20)}`}
          content={block}
          basePath={basePath}
        />
      ))}

      {/* Render streaming block */}
      {streamingBlock && (
        lastBlockStable ? (
          <MemoizedBlock
            key={`streaming-stable`}
            content={streamingBlock}
            basePath={basePath}
          />
        ) : (
          <StreamingBlock content={streamingBlock} />
        )
      )}
    </div>
  );
});

/**
 * Memoized markdown block - only re-renders when content changes
 */
const MemoizedBlock = memo(function MemoizedBlock({
  content,
  basePath,
}: {
  content: string;
  basePath?: string;
}) {
  return (
    <MarkdownContent
      content={content}
      className="[&>*:first-child]:mt-0 [&>*:last-child]:mb-0"
      basePath={basePath}
    />
  );
}, (prev, next) => prev.content === next.content && prev.basePath === next.basePath);

/**
 * Plain text streaming block - minimal DOM updates
 */
const StreamingBlock = memo(function StreamingBlock({
  content,
}: {
  content: string;
}) {
  return (
    <p className="my-2 whitespace-pre-wrap">{content}</p>
  );
});

export default StreamingMarkdown;
