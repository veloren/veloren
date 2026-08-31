-- Make all character names longer than 20 chracters be 20 characters.
--
-- In code we also limit it to 20 characters with `common::character::MAX_NAME_LENGTH`.
UPDATE "character"
    SET alias = substr(alias, 1, 20)
    WHERE length(alias) > 20;
