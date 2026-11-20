**FREE
// Native unit NRSLTST (S2.166): SET RESULT SETS ARRAY, the module captured
// on PUB400 as GN166M, called through an SQL procedure.
// Source: golden capture tools/golden/native166 n166_resultset (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt nomain;
dcl-proc listarr export;
  dcl-pi *n;
    n int(10);
  end-pi;
  dcl-ds rows qualified dim(3);
    name char(5);
    qty packed(5:2);
    note varchar(8);
  end-ds;
  rows(1).name = 'ab';
  rows(1).qty = 7.5;
  rows(1).note = 'one';
  rows(2).name = 'cd';
  rows(2).qty = -8;
  rows(2).note = '';
  rows(3).name = 'ef';
  rows(3).qty = 9;
  rows(3).note = 'three';
  exec sql set result sets array :rows for :n rows;
end-proc;
