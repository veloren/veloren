-- This migration downgrades the capacity of existing player inventories from 36 to 18. ITEMS WILL BE REMOVED.
UPDATE
    inventory
SET
    items = json_object(
        'amount',
        (
            SELECT
                json_extract(items, '$.amount')
            from
                inventory
        ),
        'slots',
        json_remove(
            (
                SELECT
                    json_extract(items, '$.slots')
                from
                    inventory
            ),
            '$[35]',
            '$[34]',
            '$[33]',
            '$[32]',
            '$[31]',
            '$[30]',
            '$[29]',
            '$[28]',
            '$[27]',
            '$[26]',
            '$[25]',
            '$[25]',
            '$[24]',
            '$[23]',
            '$[22]',
            '$[21]',
            '$[20]',
            '$[19]',
            '$[18]'
        )
    );