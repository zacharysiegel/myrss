-- Create user_read_items table for tracking read status
CREATE TABLE user_read_items (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    item_id UUID NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    is_read BOOLEAN NOT NULL DEFAULT TRUE,
    read_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, item_id)
);

-- Create indexes for performance
CREATE INDEX idx_user_read_items_user ON user_read_items(user_id);
CREATE INDEX idx_user_read_items_item ON user_read_items(item_id);