**FREE
// Stage 2, S2.91: data imports and exports. The module of GN91MAIN:
// TOTAL is exported here (INZ 100), LABEL imported from the main module.
// Captured on PUB400 (tools/golden/native91.txt n91_data).
ctl-opt nomain;
dcl-s total packed(7:0) export inz(100);
dcl-s label char(10) import;
dcl-proc addIt export;
  dcl-pi *n;
    n packed(7:0) const;
  end-pi;
  total += n;
  label = 'mod' + %char(n) + %trim(label);
end-proc;
