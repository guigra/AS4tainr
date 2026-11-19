**FREE
// Stage 2, S2.97: EXECUTE of a statement that was never prepared, with
// a USING host variable that holds invalid packed data (x'FFFF' through
// an overlay): SQLCODE and SQLSTATE, and whether the program goes on.
// Captured on PUB400 (tools/golden/native97.txt n97_execute) for the
// native unit NSQLEXE.
ctl-opt dftactgrp(*no);
dcl-ds bad;
  raw char(2) pos(1);
  n packed(3:0) pos(1);
end-ds;
dcl-s line varchar(80);
raw = x'FFFF';
monitor;
  exec sql execute s using :n;
  line = 'C ' + %char(sqlcode) + ' ' + sqlstate;
on-error;
  line = 'E ' + %char(%status);
endmon;
say(line);
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
