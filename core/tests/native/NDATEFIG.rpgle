**FREE
// Stage 2, S2.52: *LOVAL and *HIVAL assigned to date fields, for parity
// between the engines only (native_diff.rs). IBM i stores 0001-01-01 and
// 9999-12-31 (12/31/9999 in *USA) (golden capture tools/golden/native53.txt
// n53_date, PUB400, IBM i 7.5, 2026-11-16); the interpreter fills the
// field with x'00' and x'FF' bytes, and the native engine does the same
// (fidelity.md).
ctl-opt dftactgrp(*no);
dcl-s iso date(*iso);
dcl-s usa date(*usa);
iso = *loval;
dsply ('LOVAL ' + %char(iso));
iso = *hival;
dsply ('HIVAL ' + %char(iso));
usa = *hival;
dsply ('USA HIVAL ' + %char(usa));
*inlr = *on;
