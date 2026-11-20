**FREE
// Stage 2, S2.166: SET RESULT SETS ARRAY of a procedure's local data
// structure array, for a count passed as a parameter. The procedure is
// called from SQL (CREATE PROCEDURE ... EXTERNAL NAME) and its result set
// shown by Qshell's db2. Captured on PUB400 (tools/golden/native166.txt
// n166_resultset) for the native unit NRSLTST.
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
