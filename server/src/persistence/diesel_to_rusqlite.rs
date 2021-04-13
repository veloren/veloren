use crate::persistence::{error::PersistenceError, VelorenConnection};
use rusqlite::NO_PARAMS;
use tracing::{debug, info};

/// Performs a one-time migration from diesel to refinery migrations. Copies
/// diesel's __diesel_schema_migrations table records to refinery_schema_history
/// and drops __diesel_schema_migrations.
// At some point in the future, when it is deemed no longer necessary to
// support migrations from pre-rusqlite databases this method should be deleted.
pub(crate) fn migrate_from_diesel(
    connection: &mut VelorenConnection,
) -> Result<(), PersistenceError> {
    let transaction = connection
        .connection
        .transaction()
        .expect("failed to start transaction");
    #[rustfmt::skip]
    let mut stmt = transaction.prepare("
        SELECT  COUNT(1)
        FROM    sqlite_master
        WHERE   type='table'
        AND     name='__diesel_schema_migrations';
    ",
    )?;

    let diesel_migrations_table_exists = stmt.query_row(NO_PARAMS, |row| {
        let row_count: i32 = row.get(0)?;
        Ok(row_count > 0)
    })?;
    drop(stmt);

    if !diesel_migrations_table_exists {
        debug!(
            "__diesel_schema_migrations table does not exist, skipping diesel to refinery \
             migration"
        );
        return Ok(());
    }

    #[rustfmt::skip]
    transaction.execute_batch("
        -- Create temporary table to store Diesel > Refinery mapping data in
        CREATE TEMP TABLE IF NOT EXISTS _migration_map (
            diesel_version VARCHAR(50) NOT NULL,
            refinery_version INT4 NOT NULL,
            refinery_name VARCHAR(255) NOT NULL,
            refinery_checksum VARCHAR(255) NOT NULL
        );

        -- Insert mapping records to _migration_map
        INSERT INTO _migration_map VALUES ('20200411202519',1,'character','18301154882232874638');
        INSERT INTO _migration_map VALUES ('20200419025352',2,'body','6687048722955029379');
        INSERT INTO _migration_map VALUES ('20200420072214',3,'stats','2322064461300660230');
        INSERT INTO _migration_map VALUES ('20200524235534',4,'race_species','16440275012526526388');
        INSERT INTO _migration_map VALUES ('20200527145044',5,'inventory','13535458920968937266');
        INSERT INTO _migration_map VALUES ('20200528210610',6,'loadout','18209914188629128082');
        INSERT INTO _migration_map VALUES ('20200602210738',7,'inv_increase','3368217138206467823');
        INSERT INTO _migration_map VALUES ('20200703194516',8,'skills','9202176632428664476');
        INSERT INTO _migration_map VALUES ('20200707201052',9,'add_missing_inv_loadout','9127886123837666846');
        INSERT INTO _migration_map VALUES ('20200710162552',10,'dash_melee_energy_cost_fix','14010543160640061685');
        INSERT INTO _migration_map VALUES ('20200716044718',11,'migrate_armour_stats','1617484395098403184');
        INSERT INTO _migration_map VALUES ('20200719223917',12,'update_item_stats','12571040280459413049');
        INSERT INTO _migration_map VALUES ('20200724191205',13,'fix_projectile_stats','5178981757717265745');
        INSERT INTO _migration_map VALUES ('20200729204534',14,'power_stat_for_weapons','17299186713398844906');
        INSERT INTO _migration_map VALUES ('20200806212413',15,'fix_various_problems','17258097957115914749');
        INSERT INTO _migration_map VALUES ('20200816130513',16,'item_persistence','18222209741267759587');
        INSERT INTO _migration_map VALUES ('20200925200504',17,'move_sceptres','8956411670404874637');
        INSERT INTO _migration_map VALUES ('20201107182406',18,'rename_npcweapons','10703468376963165521');
        INSERT INTO _migration_map VALUES ('20201116173524',19,'move_waypoint_to_stats','10083555685813984571');
        INSERT INTO _migration_map VALUES ('20201128205542',20,'item_storage','11912657465469442777');
        INSERT INTO _migration_map VALUES ('20201213172324',21,'shinygem_to_diamond','7188502861698656149');
        INSERT INTO _migration_map VALUES ('20210124141845',22,'skills','1249519966980586191');
        INSERT INTO _migration_map VALUES ('20210125202618',23,'purge_duplicate_items','10597564860189510441');
        INSERT INTO _migration_map VALUES ('20210212054315',24,'remove_duplicate_possess_stick','10774303849135897742');
        INSERT INTO _migration_map VALUES ('20210220191847',25,'starter_gear','7937903884108396352');
        INSERT INTO _migration_map VALUES ('20210224230149',26,'weapon_replacements','16314806319051099277');
        INSERT INTO _migration_map VALUES ('20210301053817',27,'armor_reorganization','17623676960765703100');
        INSERT INTO _migration_map VALUES ('20210302023541',28,'fix_sturdy_red_backpack','10808562637001569925');
        INSERT INTO _migration_map VALUES ('20210302041950',29,'fix_other_backpacks','3143452502889073613');
        INSERT INTO _migration_map VALUES ('20210302182544',30,'fix_leather_set','5238543158379875836');
        INSERT INTO _migration_map VALUES ('20210303195917',31,'fix_debug_armor','13228825131487923091');
        INSERT INTO _migration_map VALUES ('20210306213310',32,'reset_sceptre_skills','626800208872263587');
        INSERT INTO _migration_map VALUES ('20210329012510',33,'fix_amethyst_staff','11008696478673746982');

        -- Create refinery_schema_history table
        CREATE TABLE refinery_schema_history (
            version INT4 PRIMARY KEY,
            name VARCHAR(255),
            applied_on VARCHAR(255),
            checksum VARCHAR(255)
        );

        -- Migrate diesel migration records to refinery migrations table
        INSERT INTO refinery_schema_history
        SELECT	m.refinery_version,
                m.refinery_name,
                '2021-03-27T00:00:00.000000000+00:00',
                m.refinery_checksum
        FROM 	_migration_map m
        JOIN 	__diesel_schema_migrations d ON (d.version = m.diesel_version);

        DROP TABLE __diesel_schema_migrations;"
    )?;

    transaction.commit()?;
    info!("Successfully performed one-time diesel to refinery migration");

    Ok(())
}
