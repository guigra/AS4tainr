**FREE
// Phase 2: *ISO date and time literals under DATFMT(*EUR) TIMFMT(*USA)
// in INZ (compile only; RNF0305/RNF0306 seen on 2026-11-19 but not kept).
ctl-opt dftactgrp(*no) datfmt(*eur) timfmt(*usa);
dcl-s d date inz(d'2026-01-31');
dcl-s t time inz(t'09.15.00');
*inlr = *on;
