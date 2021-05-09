-- Sets active_item to active_mainhand and second_item to inactive_mainhand.
--
-- second_item becomes inactive_mainhand because active_offhand is enforced to be 1h
-- and second_item was not necessarily guaranteed to be 1h.
UPDATE item
SET position = 'active_mainhand' WHERE position = 'active_item';
UPDATE item
SET position = 'inactive_mainhand' WHERE position = 'second_item';