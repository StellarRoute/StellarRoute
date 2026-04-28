-- Add adaptive priority to the route computation job queue.
--
-- Priority levels (lower number = higher priority):
--   0 = critical  – large-amount or explicitly elevated requests
--   1 = high      – medium-amount requests
--   2 = normal    – standard requests (default)
--   3 = low       – batch / background requests
--
-- The dequeue query orders by (priority ASC, created_at ASC) so that
-- higher-priority jobs are always processed first within the same
-- priority band, while FIFO ordering is preserved within each band.
--
-- A virtual_time column is maintained by the scheduler to implement
-- weighted-fair-queuing starvation prevention: lower-priority jobs
-- accumulate virtual time more slowly, ensuring they are eventually
-- served even under sustained high-priority load.

ALTER TABLE route_computation_jobs
  ADD COLUMN IF NOT EXISTS priority       SMALLINT     NOT NULL DEFAULT 2,
  ADD COLUMN IF NOT EXISTS virtual_time   BIGINT       NOT NULL DEFAULT 0;

-- Constraint: only accept known priority values
ALTER TABLE route_computation_jobs
  DROP CONSTRAINT IF EXISTS chk_priority_range;
ALTER TABLE route_computation_jobs
  ADD CONSTRAINT chk_priority_range CHECK (priority BETWEEN 0 AND 3);

-- Drop the old FIFO dequeue index and replace with a priority-aware one.
DROP INDEX IF EXISTS idx_route_jobs_dequeue;

CREATE INDEX IF NOT EXISTS idx_route_jobs_priority_dequeue
  ON route_computation_jobs(priority ASC, virtual_time ASC, created_at ASC)
  WHERE status = 'pending';

-- Update the general status index to include priority for efficient scans.
DROP INDEX IF EXISTS idx_route_jobs_status;
CREATE INDEX IF NOT EXISTS idx_route_jobs_status
  ON route_computation_jobs(status, priority ASC, created_at ASC);

COMMENT ON COLUMN route_computation_jobs.priority IS
  '0=critical, 1=high, 2=normal (default), 3=low';
COMMENT ON COLUMN route_computation_jobs.virtual_time IS
  'Weighted-fair-queuing virtual clock; prevents starvation of lower-priority jobs';
