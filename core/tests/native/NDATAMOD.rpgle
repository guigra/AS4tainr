**FREE
// Native unit NDATAIMP's module (S2.91): the module captured on PUB400
// as GN91MOD (tools/golden/native91 n91_data, IBM i 7.5, 2026-11-18).
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
