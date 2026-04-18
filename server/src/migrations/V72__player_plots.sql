-- Adds a 'plot' column to the character table to persist each character's
-- claimed player build area. The value is stored as a JSON string with the
-- same optional-field approach used by the 'waypoint' column so that NULL
-- means "no plot claimed" and missing sub-fields are treated as their
-- defaults.
ALTER TABLE character ADD COLUMN plot TEXT;
