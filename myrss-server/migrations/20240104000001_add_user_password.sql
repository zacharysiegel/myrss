-- Add password_hash column to users table
ALTER TABLE users ADD COLUMN password_hash VARCHAR(255) NOT NULL DEFAULT '';

-- Remove the default after adding the column
ALTER TABLE users ALTER COLUMN password_hash DROP DEFAULT;