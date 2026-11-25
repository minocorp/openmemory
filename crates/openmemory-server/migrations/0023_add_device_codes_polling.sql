ALTER TABLE device_codes
  ADD COLUMN denied_at timestamptz,
  ADD COLUMN last_polled_at timestamptz;
