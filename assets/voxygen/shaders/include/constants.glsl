/* NOTE: When included, this file will contain values for the automatically defined settings specified below. */

/* TODO: Add the ability to control the tendency to do stuff in the vertex vs. fragment shader.
 * Currently this flag is ignored and always set to prefer fragment, but this tradeoff is not correct on all
 * machines in all cases (mine, for instance). */
#define VOXYGEN_COMPUTATION_PREFERENCE_FRAGMENT 0
#define VOXYGEN_COMPUTATION_PREFERENCE_VERTEX 1

#define FLUID_MODE_LOW 0
#define FLUID_MODE_MEDIUM 1
#define FLUID_MODE_HIGH 2

#define REFLECTION_MODE_LOW 0
#define REFLECTION_MODE_MEDIUM 1
#define REFLECTION_MODE_HIGH 2

#define CLOUD_MODE_NONE 0
#define CLOUD_MODE_MINIMAL 1
#define CLOUD_MODE_LOW 2
#define CLOUD_MODE_MEDIUM 3
#define CLOUD_MODE_HIGH 4
#define CLOUD_MODE_ULTRA 5

#define LIGHTING_ALGORITHM_LAMBERTIAN 0
#define LIGHTING_ALGORITHM_BLINN_PHONG 1
#define LIGHTING_ALGORITHM_ASHIKHMIN 2

#define SHADOW_MODE_NONE 0
#define SHADOW_MODE_CHEAP 1
#define SHADOW_MODE_MAP 2

/* Unlike the other flags (for now anyway), these are bitmask values */
#define LIGHTING_TYPE_REFLECTION 0x01
#define LIGHTING_TYPE_TRANSMISSION 0x02

/* Currently ignored, but ideally shoud be helpful for determining light transport properties. */
#define LIGHTING_REFLECTION_KIND_DIFFUSE 0
#define LIGHTING_REFLECTION_KIND_GLOSSY 1
#define LIGHTING_REFLECTION_KIND_SPECULAR 2

#define LIGHTING_TRANSPORT_MODE_IMPORTANCE 0
/* Radiance mode is currently used as a proxy for "attenuation and medium materials
 * matter," but we may make it more granular. */
#define LIGHTING_TRANSPORT_MODE_RADIANCE 1

#define LIGHTING_DISTRIBUTION_SCHEME_MICROFACET 0
#define LIGHTING_DISTRIBUTION_SCHEME_VOXEL 1

#define LIGHTING_DISTRIBUTION_BECKMANN 0
#define LIGHTING_DISTRIBUTION_TROWBRIDGE 1

#define MEDIUM_AIR 0
#define MEDIUM_WATER 1

#define MAT_SKY 0
#define MAT_BLOCK 1
#define MAT_FLUID 2
#define MAT_FIGURE 3
#define MAT_LOD 4

// An arbitrary value that represents a very far distance (at least as far as the player should be able to see) without
// being too far that we end up with precision issues (used in clouds and elsewhere).
#define DIST_CAP 50000

/* Constants expected to be defined automatically by configuration: */

/*
#define VOXYGEN_COMPUTATION_PREFERENCE <preference>
#define FLUID_MODE <mode>
#define CLOUD_MODE <mode>
#define LIGHTING_ALGORITHM <algorithm>
#define SHADOW_MODE <mode>
*/

/* Constants possibly defined automatically by configuration: */

/*
#define POINT_GLOW_FACTOR <0.0..1.0>
*/

/* Constants expected to be defined by any shader that needs to perform lighting calculations
 * (but whose values may take automatically defined constants into account): */

/*
// At least one of LIGHTING_TYPE_REFLECTION or LIGHTING_TYPE_TRANSMISSION should be set.
#define LIGHTING_TYPE <type bitmask>
#define LIGHTING_REFLECTION_KIND <kind>
#define LIGHTING_TRANSPORT_MODE <mode>
#define LIGHTING_DISTRIBUTION_SCHEME <scheme>
#define LIGHTING_DISTRIBUTION <distribution>
*/

/* Constants that *may* be defined by any shader.
 * (and whose values may take automatically defined constants into account): */

/*
// When sets, shadow maps are used to cast shadows.
#define HAS_SHADOW_MAPS
// When set, "full" LOD terrain informatino is available (e.g. terrain colors).
#define HAS_LOD_FULL_INFO
*/
