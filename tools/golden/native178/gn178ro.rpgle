**FREE
// Stage 2, S2.178: SET RESULT SETS ARRAY of a data structure of multiple
// occurrences, for a count in a field: each occurrence a row. The
// procedure is called from SQL (CREATE PROCEDURE ... EXTERNAL NAME) and
// its result set shown by Qshell's db2. Captured on PUB400
// (tools/golden/native178.txt n178_resultocc) for the native unit NRSLTOC.
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
