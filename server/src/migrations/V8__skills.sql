ALTER TABLE "stats" ADD COLUMN skills TEXT;

-- Update all existing stats records to "" which will cause characters to be populated
-- with the default skill groups/skills on next login.
UPDATE "stats" SET skills = '""';