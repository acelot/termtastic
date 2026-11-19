ALTER TABLE messages ADD COLUMN delivery_state INTEGER;

UPDATE messages
SET delivery_state = CASE
    WHEN routing_error = 0 THEN 4 -- DeliveryState::DeliveredToMesh
    ELSE routing_error + 32       -- DeliveryState::RoutingError
END
WHERE routing_error IS NOT NULL;

ALTER TABLE messages DROP COLUMN routing_error;