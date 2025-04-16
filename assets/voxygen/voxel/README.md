### Voxel model organization

This is a description of the current organization to act as a guide when adding new models. Please update this when re-organizing the folder structure.

* `armor` - Equipable clothing/armor items.
* `figure` - Models used to display entities with `Body::Humanoid`.
* `glider` - Equipable glider items.
* `item` - Models for items that don't fall into `armor`, `glider`, `lantern`, or `weapon`.
* `lantern` - Equipable lantern items.
* `object` - Models used to display entities with `Body::Object` that aren't projectiles and aren't shared with other purposes such as sprites.
* `sprite` - Models used for terrain sprites. If the model is shared with other uses such as an item, always put it in the `sprite` folder since this is a narrower category with more constraints on the model. All models used in `sprite_manifest.ron` will be in this folder.
* `npc` -  Models used to display entities with the `Body` component except `Body::Humanoid`, `Body::Object`, and `Body::ItemDrop`.
* `weapon` - Mainly items equipable to hand slots, projectiles, and weapon components.
