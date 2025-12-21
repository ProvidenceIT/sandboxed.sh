-- Migration: Add final_tree column to missions table
-- This stores the agent execution tree when a mission completes,
-- allowing users to view the tree for finished missions.

-- Add the column (JSONB for efficient storage of tree structure)
ALTER TABLE missions ADD COLUMN IF NOT EXISTS final_tree JSONB;

-- Add a comment explaining the column
COMMENT ON COLUMN missions.final_tree IS 'Agent execution tree snapshot saved when mission completes (for visualization)';
