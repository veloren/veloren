-- Temp table relating earned_sp to the experience required to earn that amount
CREATE TEMP TABLE _sp_series
(
	earned_sp INT NOT NULL,
	exp INT NOT NULL
);

-- Inserts exp values corresponding to the sp value
INSERT INTO _sp_series
WITH RECURSIVE sp_series(earned_sp, exp) AS (
    SELECT 0, 0
    UNION ALL
    -- Function is the same as function in skillset/mod.rs in fn skill_point_cost as of time of this migration
    -- though slightly modified to account for sqlite lacking functions for floor and exp
    -- Floor modification is replacing floor(a) with round(a - 0.5)
    -- Exp mofidication is replacing exp(-a) with 1 / (2^(a*1.442695)) where 1.442695 = log(e)/log(2)
    SELECT earned_sp + 1,
           exp +
			CASE
				WHEN earned_sp < 300
					THEN (10 * ROUND(((1000.0 / 10.0) / (1.0 + 1.0 / (1 << ROUND((0.125 * (earned_sp + 1) * 1.442695) - 0.1)) * (1000.0 / 70.0 - 1.0))) - 0.5))
				ELSE
					1000
			END
    FROM sp_series
    -- Only create table up to maximum value of earned_sp in database
    WHERE earned_sp <= (SELECT MAX(earned_sp) FROM skill_group)
)
SELECT	earned_sp,
		exp
FROM sp_series;

-- Update exp column with new values, add the leftover exp to this value
UPDATE skill_group
SET exp = skill_group.exp + (SELECT exp FROM _sp_series WHERE earned_sp = skill_group.earned_sp);

-- Progress in earned_sp is tracked in exp now
ALTER TABLE skill_group DROP COLUMN earned_sp;
-- available_sp is now useless to track, automatically recalculated when loading persisted skill groups
ALTER TABLE skill_group DROP COLUMN available_sp;
-- Skills are now tracked in skill_group table as a json blob. Json blob fine since we can just invalidate and let people respec if it doesn't deserialize
ALTER TABLE skill_group ADD COLUMN skills TEXT DEFAULT "" NOT NULL;
-- Skills now tracked in skill_group table, can ust drop
DROP TABLE skill;
-- Temp table no longer needed, drop it
DROP TABLE _sp_series;