**FREE
// Stage 2, S2.168: %DEC of a date field whose bytes are blanks (the
// field overlaid by a character field in a data structure), in a MONITOR
// group: the status. Then %DEC of a valid date. Captured on PUB400
// (tools/golden/native168.txt n168_decbad) for the native unit NDECBAD.
ctl-opt dftactgrp(*no);
dcl-ds x;
  c char(10) pos(1) inz(*blanks);
  d date pos(1);
end-ds;
dcl-s n packed(9:0);
dcl-s line varchar(80);

monitor;
  n = %dec(d);
  line = 'no error ' + %char(n);
on-error;
  line = %char(%status);
endmon;
c = '2026-01-31';
n = %dec(d);
line += ' ' + %char(n);
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
