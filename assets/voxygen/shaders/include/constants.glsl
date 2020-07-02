/* NOTE: When included, this file will contain values for the automatically defined settings specified below. */

/* TODO: Add the ability to control the tendency to do stuff in the vertex vs. fragment shader.
 * Currently this flag is ignored and always set to prefer fragment, but this tradeoff is not correct on all
 * machines in all cases (mine, for instance). */
#define VOXYGEN_COMPUTATION_PREERENCE_FRAGMENT 0u
#define VOXYGEN_COMPUTATION_PREERENCE_VERTEX 1u

#define FLUID_MODE_CHEAP 0u
#define FLUID_MODE_SHINY 1u

#define CLOUD_MODE_NONE 0u
#define CLOUD_MODE_REGULAR 1u

#define LIGHTING_ALGORITHM_LAMBERTIAN 0u
#define LIGHTING_ALGORITHM_BLINN_PHONG 1u
#define LIGHTING_ALGORITHM_ASHIKHMIN 2u

#define SHADOW_MODE_NONE 0u
#define SHADOW_MODE_CHEAP 1u
#define SHADOW_MODE_MAP 2u

/* Unlike the other flags (for now anyway), these are bitmask values */
#define LIGHTING_TYPE_REFLECTION 0x01u
#define LIGHTING_TYPE_TRANSMISSION 0x02u

/* Currently ignored, but ideally shoud be helpful for determining light transport properties. */
#define LIGHTING_REFLECTION_KIND_DIFFUSE 0u
#define LIGHTING_REFLECTION_KIND_GLOSSY 1u
#define LIGHTING_REFLECTION_KIND_SPECULAR 2u

#define LIGHTING_TRANSPORT_MODE_IMPORTANCE 0u
/* Radiance mode is currently used as a proxy for "attenuation and medium materials
 * matter," but we may make it more granular. */
#define LIGHTING_TRANSPORT_MODE_RADIANCE 1u

#define LIGHTING_DISTRIBUTION_SCHEME_MICROFACET 0u
#define LIGHTING_DISTRIBUTION_SCHEME_VOXEL 1u

#define LIGHTING_DISTRIBUTION_BECKMANN 0u
#define LIGHTING_DISTRIBUTION_TROWBRIDGE 1u

/* Constants expected to be defined automatically by configuration: */

/*
#define VOXYGEN_COMPUTATION_PREERENCE <preference>
#define FLUID_MODE <mode>
#define CLOUD_MODE <mode>
#define LIGHTING_ALGORITHM <algorithm>
#define SHADOW_MODE <mode>
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
*/
