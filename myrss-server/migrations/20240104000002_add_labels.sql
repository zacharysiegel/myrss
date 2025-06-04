-- Create labels table
CREATE TABLE labels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    color VARCHAR(7) DEFAULT '#3b82f6',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, name)
);

-- Create subscription_labels junction table
CREATE TABLE subscription_labels (
    subscription_id UUID NOT NULL REFERENCES subscriptions(id) ON DELETE CASCADE,
    label_id UUID NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    PRIMARY KEY (subscription_id, label_id)
);

-- Remove folder column from subscriptions
ALTER TABLE subscriptions DROP COLUMN IF EXISTS folder;

-- Create indexes
CREATE INDEX idx_labels_user_id ON labels(user_id);
CREATE INDEX idx_subscription_labels_subscription ON subscription_labels(subscription_id);
CREATE INDEX idx_subscription_labels_label ON subscription_labels(label_id);