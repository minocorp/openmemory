SET LOCAL lock_timeout = '5s';

-- squawk-ignore adding-not-nullable-field
ALTER TABLE users ALTER COLUMN username SET NOT NULL;

-- squawk-ignore adding-not-nullable-field
ALTER TABLE users ALTER COLUMN display_name SET NOT NULL;

-- squawk-ignore adding-not-nullable-field
ALTER TABLE users ALTER COLUMN user_handle SET NOT NULL;
