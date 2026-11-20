**FREE
// Native unit NRSLTOC (S2.178): SET RESULT SETS ARRAY of an OCCURS
// structure, the module captured on PUB400 as GN178M, called through an
// SQL procedure.
// Source: golden capture tools/golden/native178 n178_resultocc (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt nomain;
dcl-ds out occurs(4);
  line char(6);
  qty packed(3:0);
end-ds;
dcl-s count packed(5:0) inz(2);
dcl-proc listocc export;
  dcl-pi *n;
  end-pi;
  %occur(out) = 1;
  line = 'first';
  qty = 5;
  %occur(out) = 2;
  line = 'second';
  qty = -7;
  %occur(out) = 3;
  line = 'third';
  qty = 1;
  exec sql set result sets array :out for :count rows;
end-proc;
