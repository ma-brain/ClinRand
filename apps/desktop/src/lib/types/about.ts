/** Response shape from the Tauri `get_about` command. */
export interface AboutInfo {
  engine_version: string;
  algo_version: number;
  rng_algorithm: string;
  rng_crate: string;
  rng_crate_version: string;
  disclaimer: string;
}
