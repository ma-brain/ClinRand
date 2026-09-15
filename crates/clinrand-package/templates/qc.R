#!/usr/bin/env Rscript
# ClinRand independent QC script (plan §9.1).
#
# HOW TO RUN
#   Change into the package directory (the folder that contains
#   manifest.unblinded.json, list.csv, and stream.csv) and run:
#
#       Rscript qc.R
#
#   The script reads manifest.unblinded.json, list.csv, and stream.csv from the
#   current working directory using relative paths. It does not take arguments.
#
# WHAT IT PROVES
#   It rebuilds every treatment assignment from the recorded random draws in
#   stream.csv (it does NOT reimplement ChaCha20) and compares the result to
#   list.csv row by row, re-derives config_sha256 with the same canonical-JSON
#   rules the engine uses, verifies the list.csv / stream.csv content hashes,
#   and re-checks properties P01-P09. Any required failure exits non-zero.
#
# DEPENDENCIES
#   Base R plus jsonlite and digest only. No other packages are permitted.
#
# THE SEED
#   This script never prints the seed. It has no need for seed_hex and does not
#   read it. Do not add code that cats or logs any seed material.
#
# ---------------------------------------------------------------------------
# Study-specific header (interpolated by clinrand-package::render_qc_r). These
# concrete values are for the human reviewer and are cross-checked against the
# manifest below; the manifest config remains the source of truth for the QC.
# @@CLINRAND_HEADER@@
# ---------------------------------------------------------------------------

suppressWarnings(suppressMessages({
  ok_json <- requireNamespace("jsonlite", quietly = TRUE)
  ok_dig <- requireNamespace("digest", quietly = TRUE)
}))
if (!ok_json || !ok_dig) {
  cat("[FAIL] setup: qc.R requires the 'jsonlite' and 'digest' packages.\n")
  cat("       Install with: install.packages(c(\"jsonlite\", \"digest\"))\n")
  quit(status = 1L)
}

# ---- result accumulation --------------------------------------------------

.results <- new.env(parent = emptyenv())
.results$rows <- list()

record <- function(id, passed, detail) {
  status <- if (isTRUE(passed)) "PASS" else "FAIL"
  cat(sprintf("[%s] %s: %s\n", status, id, detail))
  .results$rows[[length(.results$rows) + 1L]] <- isTRUE(passed)
  invisible(NULL)
}

info <- function(id, detail) {
  cat(sprintf("[INFO] %s: %s\n", id, detail))
  invisible(NULL)
}

fatal <- function(msg) {
  cat(sprintf("[FAIL] fatal: %s\n", msg))
  cat("\nOVERALL: FAIL\n")
  print(sessionInfo())
  quit(status = 1L)
}

# ---- canonical JSON (mirror of clinrand-package canonical.rs) --------------

canon_string <- function(s) {
  cps <- utf8ToInt(enc2utf8(s))
  out <- vapply(cps, function(cp) {
    if (cp == 0x22L) "\\\""
    else if (cp == 0x5CL) "\\\\"
    else if (cp == 0x08L) "\\b"
    else if (cp == 0x0CL) "\\f"
    else if (cp == 0x0AL) "\\n"
    else if (cp == 0x0DL) "\\r"
    else if (cp == 0x09L) "\\t"
    else if (cp < 0x20L) sprintf("\\u%04x", cp)
    else intToUtf8(cp)
  }, character(1))
  paste0("\"", paste0(out, collapse = ""), "\"")
}

canon_number <- function(x) {
  if (!is.finite(x) || x != round(x)) {
    fatal("canonical JSON numbers must be integers")
  }
  sprintf("%.0f", as.double(x))
}

# Recursively canonicalize a jsonlite value parsed with simplifyVector = FALSE:
# named list -> JSON object (keys sorted by UTF-8 byte order), unnamed list ->
# JSON array (order preserved), scalars -> string/number/bool. Empty list is
# treated as an empty array (StudyConfig has no empty objects).
canon <- function(x) {
  if (is.list(x)) {
    nm <- names(x)
    if (is.null(nm) || all(nm == "")) {
      if (length(x) == 0L) return("[]")
      parts <- vapply(x, canon, character(1))
      return(paste0("[", paste(parts, collapse = ","), "]"))
    }
    ord <- order(nm, method = "radix")
    parts <- vapply(ord, function(i) {
      paste0(canon_string(nm[i]), ":", canon(x[[i]]))
    }, character(1))
    return(paste0("{", paste(parts, collapse = ","), "}"))
  }
  if (is.character(x)) return(canon_string(x))
  if (is.logical(x)) return(if (isTRUE(x)) "true" else "false")
  if (is.numeric(x)) return(canon_number(x))
  if (is.null(x)) return("null")
  fatal("value cannot be canonicalized")
}

sha256_file <- function(path) {
  n <- file.info(path)$size
  if (is.na(n)) fatal(sprintf("cannot stat %s", path))
  bytes <- readBin(path, what = "raw", n = n)
  digest::digest(bytes, algo = "sha256", serialize = FALSE)
}

sha256_string <- function(s) {
  digest::digest(charToRaw(enc2utf8(s)), algo = "sha256", serialize = FALSE)
}

# ---- load inputs ----------------------------------------------------------

for (f in c("manifest.unblinded.json", "list.csv", "stream.csv")) {
  if (!file.exists(f)) {
    fatal(sprintf("expected file '%s' not found in %s", f, getwd()))
  }
}

manifest <- jsonlite::fromJSON("manifest.unblinded.json",
                               simplifyVector = TRUE,
                               simplifyDataFrame = FALSE)
manifest_raw <- jsonlite::fromJSON("manifest.unblinded.json",
                                   simplifyVector = FALSE)
config <- manifest$config
config_raw <- manifest_raw$config

list_df <- read.csv("list.csv", colClasses = "character", check.names = FALSE)
stream_df <- read.csv("stream.csv", colClasses = "character", check.names = FALSE)

# ---- header cross-checks (interpolated constants vs manifest) --------------

header_ok <- identical(as.character(manifest$study_id), EXPECTED_STUDY_ID) &&
  identical(as.character(manifest$protocol_version), EXPECTED_PROTOCOL_VERSION) &&
  identical(as.character(config$method), EXPECTED_METHOD) &&
  as.double(config$list_length_per_stratum) == EXPECTED_LIST_LENGTH_PER_STRATUM
record("header", header_ok,
       sprintf("interpolated study_id/protocol/method/list_length match manifest (study_id=%s)",
               manifest$study_id))

# ---- config_sha256 (independent canonical JSON) ---------------------------

recomputed_config_sha <- sha256_string(canon(config_raw))
record("config_sha256", identical(recomputed_config_sha, as.character(manifest$config_sha256)),
       sprintf("recomputed canonical-config SHA-256 %s manifest value",
               if (identical(recomputed_config_sha, as.character(manifest$config_sha256))) "matches" else "differs from"))

# ---- content hashes -------------------------------------------------------

record("list_sha256", identical(sha256_file("list.csv"), as.character(manifest$list_sha256)),
       "list.csv bytes hash to manifest list_sha256")
record("stream_sha256", identical(sha256_file("stream.csv"), as.character(manifest$stream_sha256)),
       "stream.csv bytes hash to manifest stream_sha256")

# ---- reconstruction from the recorded stream ------------------------------

arms <- config$arms
arm_codes <- vapply(arms, function(a) as.character(a$code), character(1))
arm_ratios <- vapply(arms, function(a) as.double(a$ratio), numeric(1))
ratio_sum <- sum(arm_ratios)

factors <- config$strata
if (is.null(factors)) factors <- list()
factor_names <- if (length(factors) == 0L) character(0) else
  vapply(factors, function(f) as.character(f$name), character(1))

# stream cursor over accepted draws; n == 1 consumes nothing (mirrors core).
stream_n <- nrow(stream_df)
stream_bound <- if (stream_n == 0L) numeric(0) else as.double(stream_df$bound)
stream_value <- if (stream_n == 0L) numeric(0) else as.double(stream_df$value)
stream_purpose <- if (stream_n == 0L) character(0) else as.character(stream_df$purpose)
.cursor <- 1L
stream_error <- NULL

draw <- function(n, purpose) {
  if (n == 1) return(0)
  if (.cursor > stream_n) {
    if (is.null(stream_error)) stream_error <<- "stream exhausted before reconstruction finished"
    return(0)
  }
  b <- stream_bound[.cursor]
  v <- stream_value[.cursor]
  p <- stream_purpose[.cursor]
  if (b != n || !identical(p, purpose)) {
    if (is.null(stream_error)) {
      stream_error <<- sprintf("stream row %d: expected bound=%g purpose=%s, found bound=%g purpose=%s",
                               .cursor, n, purpose, b, p)
    }
  }
  .cursor <<- .cursor + 1L
  v
}

fisher_yates <- function(items) {
  len <- length(items)
  if (len <= 1L) return(items)
  for (i in len:2L) {
    j0 <- draw(i, "permutation")
    ri <- i
    rj <- j0 + 1L
    tmp <- items[ri]
    items[ri] <- items[rj]
    items[rj] <- tmp
  }
  items
}

arm_for_simple_draw <- function(d) {
  cumulative <- 0
  for (k in seq_along(arm_codes)) {
    cumulative <- cumulative + arm_ratios[k]
    if (d < cumulative) return(arm_codes[k])
  }
  arm_codes[length(arm_codes)]
}

build_multiset <- function(block_size) {
  unit <- block_size / ratio_sum
  out <- character(0)
  for (k in seq_along(arm_codes)) {
    out <- c(out, rep(arm_codes[k], arm_ratios[k] * unit))
  }
  out
}

# canonical stratum order: config factor order, last factor varies fastest.
strata_combos <- function() {
  if (length(factors) == 0L) return(list(stats::setNames(list(), character(0))))
  count <- 1L
  for (f in factors) count <- count * length(f$levels)
  combos <- vector("list", count)
  for (ci in 0:(count - 1L)) {
    remaining <- ci
    combo <- list()
    for (f in rev(factors)) {
      lc <- length(f$levels)
      li <- remaining %% lc
      remaining <- remaining %/% lc
      combo[[as.character(f$name)]] <- as.character(f$levels[li + 1L])
    }
    combos[[ci + 1L]] <- combo
  }
  combos
}

combos <- strata_combos()
n_strata <- length(combos)
L <- as.double(config$list_length_per_stratum)
method <- as.character(config$method)

combo_key <- function(combo) {
  if (length(factor_names) == 0L) return("")
  paste(vapply(factor_names, function(fn) as.character(combo[[fn]]), character(1)),
        collapse = "\x1f")
}

row_key <- function() {
  if (length(factor_names) == 0L) return(rep("", nrow(list_df)))
  cols <- lapply(factor_names, function(fn) as.character(list_df[[fn]]))
  do.call(paste, c(cols, list(sep = "\x1f")))
}

recon_arm <- character(0)
recon_key <- character(0)

if (method == "simple") {
  for (combo in combos) {
    ck <- combo_key(combo)
    if (L >= 1) {
      for (pos in seq_len(L)) {
        d <- draw(ratio_sum, "simple_allocation")
        recon_arm <- c(recon_arm, arm_for_simple_draw(d))
        recon_key <- c(recon_key, ck)
      }
    }
  }
} else {
  block <- config$block
  block_kind <- as.character(block$kind)
  sorted_sizes <- if (block_kind == "variable") sort(unique(as.double(block$sizes))) else NULL
  resolve_block_size <- function() {
    if (block_kind == "fixed") return(as.double(block$size))
    idx0 <- draw(length(sorted_sizes), "block_size")
    sorted_sizes[idx0 + 1L]
  }
  for (combo in combos) {
    ck <- combo_key(combo)
    remaining <- L
    while (remaining > 0) {
      bs <- resolve_block_size()
      multiset <- build_multiset(bs)
      multiset <- fisher_yates(multiset)
      keep <- min(remaining, bs)
      recon_arm <- c(recon_arm, multiset[seq_len(keep)])
      recon_key <- c(recon_key, rep(ck, keep))
      remaining <- remaining - keep
    }
  }
}

rkeys <- row_key()
list_arm <- as.character(list_df$arm_code)

# reconstruction row-wise comparison (first mismatch fails)
recon_ok <- TRUE
recon_detail <- sprintf("rebuilt %d assignments from stream.csv; all match list.csv", length(recon_arm))
if (length(recon_arm) != nrow(list_df)) {
  recon_ok <- FALSE
  recon_detail <- sprintf("reconstruction length %d != list.csv rows %d",
                          length(recon_arm), nrow(list_df))
} else {
  mism <- which(recon_arm != list_arm | recon_key != rkeys)
  if (length(mism) > 0L) {
    first <- mism[1]
    recon_ok <- FALSE
    recon_detail <- sprintf("row %d: reconstructed arm '%s' (stratum '%s') != list.csv arm '%s' (stratum '%s')",
                            first, recon_arm[first], recon_key[first], list_arm[first], rkeys[first])
  }
}
record("reconstruction", recon_ok, recon_detail)

stream_consumed_ok <- is.null(stream_error) && ((.cursor - 1L) == stream_n)
stream_detail <- if (!is.null(stream_error)) stream_error else
  sprintf("consumed %d of %d recorded draws", .cursor - 1L, stream_n)
record("stream", stream_consumed_ok, stream_detail)

# ---- property checks P01-P09 ----------------------------------------------

nrec <- nrow(list_df)
block_id <- as.integer(list_df$block_id)
block_size <- as.integer(list_df$block_size)
position_in_block <- as.integer(list_df$position_in_block)
rand_num <- as.double(list_df$randomization_number)

# P01: record count == L * n_strata
record("P01", nrec == L * n_strata,
       sprintf("record count %d vs list_length_per_stratum x n_strata (%g)", nrec, L * n_strata))

# allowed block sizes
allowed_sizes <- if (method == "simple") 1 else
  if (as.character(config$block$kind) == "fixed") as.double(config$block$size) else
    unique(as.double(config$block$sizes))
record("P02", all(block_size %in% allowed_sizes),
       sprintf("all block sizes within allowed set {%s}", paste(allowed_sizes, collapse = ",")))

# grouping by (stratum, block_id) for P03 / P08
group_keys <- paste(rkeys, block_id, sep = "\x1f")
groups <- split(seq_len(nrec), group_keys)
max_bid <- tapply(block_id, rkeys, max)

p03_ok <- TRUE
p03_detail <- "all complete ratio-bearing blocks have exact ratio counts"
p08_ok <- TRUE
p08_detail <- "position_in_block is 1..block_size (or 1..kept for truncated finals)"

for (g in groups) {
  bs <- block_size[g[1]]
  kept <- length(g)
  sk <- rkeys[g[1]]
  bid <- block_id[g[1]]
  trunc_final <- (kept < bs) && (max_bid[[sk]] == bid)

  # P08
  if (p08_ok) {
    if (kept > bs) {
      p08_ok <- FALSE
      p08_detail <- sprintf("stratum '%s'/block %d: kept %d > block_size %d", sk, bid, kept, bs)
    } else if (kept < bs && !trunc_final) {
      p08_ok <- FALSE
      p08_detail <- sprintf("stratum '%s'/block %d: non-final under-full block (kept %d < %d)", sk, bid, kept, bs)
    } else {
      expected_end <- if (trunc_final) kept else bs
      pos_sorted <- sort(position_in_block[g])
      if (!identical(pos_sorted, seq_len(expected_end)) ||
          any(position_in_block[g] < 1L) || any(position_in_block[g] > bs)) {
        p08_ok <- FALSE
        p08_detail <- sprintf("stratum '%s'/block %d: positions not 1..%d", sk, bid, expected_end)
      }
    }
  }

  # P03: only complete (kept == block_size) ratio-bearing blocks are checked;
  # the truncated final block of each stratum is exempt (mirrors core).
  if (p03_ok) {
    if (kept > bs) {
      p03_ok <- FALSE
      p03_detail <- sprintf("stratum '%s'/block %d: kept %d > block_size %d", sk, bid, kept, bs)
    } else if (kept < bs) {
      if (!trunc_final) {
        p03_ok <- FALSE
        p03_detail <- sprintf("stratum '%s'/block %d: non-final under-full block", sk, bid)
      }
    } else if (bs %% ratio_sum == 0) {
      unit <- bs / ratio_sum
      block_arms <- list_arm[g]
      for (k in seq_along(arm_codes)) {
        expected <- arm_ratios[k] * unit
        actual <- sum(block_arms == arm_codes[k])
        if (actual != expected) {
          p03_ok <- FALSE
          p03_detail <- sprintf("stratum '%s'/block %d: arm %s count %d != expected %g",
                                sk, bid, arm_codes[k], actual, expected)
          break
        }
      }
      if (p03_ok && !all(block_arms %in% arm_codes)) {
        p03_ok <- FALSE
        p03_detail <- sprintf("stratum '%s'/block %d: unexpected arm code", sk, bid)
      }
    }
  }
}
record("P03", p03_ok, p03_detail)

# P04: no duplicate randomization numbers
record("P04", !any(duplicated(list_df$randomization_number)),
       sprintf("no duplicate randomization numbers among %d records", nrec))

# P05: numbers contiguous ascending within numbering scheme
numbering <- config$numbering
num_kind <- as.character(numbering$kind)
p05_ok <- TRUE
p05_detail <- "numbers contiguous and ascending"
if (num_kind == "global") {
  start <- as.double(numbering$start)
  expected <- start + (seq_len(nrec) - 1L)
  p05_ok <- isTRUE(all(rand_num == expected))
  if (!p05_ok) p05_detail <- "global numbering not contiguous ascending"
} else {
  start <- as.double(numbering$start)
  nb <- as.double(numbering$block_size)
  for (si in seq_along(combos)) {
    idxs <- which(rkeys == combo_key(combos[[si]]))
    base <- start + (si - 1L) * nb
    exp <- base + (seq_along(idxs) - 1L)
    if (!isTRUE(all(rand_num[idxs] == exp))) {
      p05_ok <- FALSE
      p05_detail <- sprintf("per_stratum_range numbering mismatch in stratum %d", si)
      break
    }
  }
}
record("P05", p05_ok, p05_detail)

# P06: every stratum combination present (when L > 0)
present_keys <- unique(rkeys)
combo_keys_all <- vapply(combos, combo_key, character(1))
p06_ok <- (L == 0) || all(combo_keys_all %in% present_keys)
record("P06", p06_ok,
       sprintf("all %d stratum combination(s) present", n_strata))

# P07: canonical stratum order segments
p07_ok <- TRUE
p07_detail <- "each canonical stratum segment contains only that stratum's records"
if (L > 0 && nrec == L * n_strata) {
  for (si in seq_along(combos)) {
    seg <- ((si - 1L) * L + 1L):((si - 1L) * L + L)
    if (!all(rkeys[seg] == combo_keys_all[si])) {
      p07_ok <- FALSE
      p07_detail <- sprintf("stratum segment %d contains cross-stratum records", si)
      break
    }
  }
} else if (!all(rkeys %in% combo_keys_all)) {
  p07_ok <- FALSE
  p07_detail <- "record carries a stratum that is not a configured combination"
}
record("P07", p07_ok, p07_detail)

# P08 (computed above in the group loop)
record("P08", p08_ok, p08_detail)

# P09: per-stratum arm counts within one-block tolerance
if (method == "simple") {
  record("P09", TRUE, "simple randomization has no block-balance guarantee; P09 not applied")
} else {
  max_block <- if (as.character(config$block$kind) == "fixed") as.double(config$block$size) else
    max(as.double(config$block$sizes))
  p09_ok <- TRUE
  p09_detail <- sprintf("per-stratum arm counts within one-block tolerance (max_block=%g)", max_block)
  for (ck in combo_keys_all) {
    idxs <- which(rkeys == ck)
    n <- length(idxs)
    members <- list_arm[idxs]
    for (k in seq_along(arm_codes)) {
      count <- sum(members == arm_codes[k])
      lhs <- abs(count * ratio_sum - n * arm_ratios[k])
      if (lhs > max_block * ratio_sum) {
        p09_ok <- FALSE
        p09_detail <- sprintf("stratum '%s': arm %s count %d of %d exceeds one-block tolerance",
                              ck, arm_codes[k], count, n)
        break
      }
    }
    if (!p09_ok) break
  }
  record("P09", p09_ok, p09_detail)
}

# P10: informational only (max identical consecutive run per stratum)
global_max <- 0L
for (ck in combo_keys_all) {
  members <- list_arm[which(rkeys == ck)]
  max_run <- 0L
  cur <- 0L
  prev <- NA_character_
  for (a in members) {
    if (!is.na(prev) && identical(a, prev)) cur <- cur + 1L else { cur <- 1L; prev <- a }
    if (cur > max_run) max_run <- cur
  }
  if (max_run > global_max) global_max <- max_run
}
info("P10", sprintf("informational only (not a failure): maximum identical consecutive arm run per stratum global_max=%d. Long runs are legitimate outcomes of correct randomization and must not trigger regeneration.",
                    global_max))

# ---- overall verdict ------------------------------------------------------

all_passed <- all(vapply(.results$rows, isTRUE, logical(1)))
cat(sprintf("\nOVERALL: %s\n", if (all_passed) "PASS" else "FAIL"))
print(sessionInfo())
if (!all_passed) quit(status = 1L)
