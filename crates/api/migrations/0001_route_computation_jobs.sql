-- Route computation job queue table
-- Enables distributed processing of route computation tasks

CREATE TABLE IF NOT EXISTS route_computation_jobs (
  id BIGSERIAL PRIMARY KEY,
  job_key TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'processing', 'completed', 'failed')),
  payload JSONB NOT NULL,
  attempt INTEGER NOT NULL DEFAULT 0,
  max_retries INTEGER NOT NULL DEFAULT 3,
  error_message TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient job queue operations
CREATE INDEX IF NOT EXISTS idx_route_jobs_status
  ON route_computation_jobs(status, created_at ASC);

CREATE INDEX IF NOT EXISTS idx_route_jobs_job_key
  ON route_computation_jobs(job_key);

CREATE INDEX IF NOT EXISTS idx_route_jobs_updated
  ON route_computation_jobs(updated_at DESC);

-- Composite index for dequeue operation (find next pending job)
CREATE INDEX IF NOT EXISTS idx_route_jobs_dequeue
  ON route_computation_jobs(status, created_at ASC)
  WHERE status = 'pending';

-- Comment for documentation
COMMENT ON TABLE route_computation_jobs IS 'Durable queue for distributed route computation tasks with deduplication support';
COMMENT ON COLUMN route_computation_jobs.job_key IS 'Unique deterministic key: route:base:quote:amount:type - enables deduplication';
COMMENT ON COLUMN route_computation_jobs.status IS 'Job lifecycle: pending -> processing -> completed/failed';
COMMENT ON COLUMN route_computation_jobs.payload IS 'Serialized RouteComputationTaskPayload with all computation parameters';
