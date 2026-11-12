**FREE
// Nested MONITOR groups, for the upstream fix of issue #3.
ctl-opt dftactgrp(*no);
dcl-s x packed(5:0) inz(0);
monitor;
  monitor;
    x = 10 / x;
  on-error 00103;
    say('MON INNER');
  endmon;
on-error 00102;
  say('MON OUTER');
endmon;
say('MON AFTER');
monitor;
  monitor;
    x = 10 / x;
  on-error 00102;
    say('INNER TAKES IT');
  endmon;
on-error;
  say('OUTER SHOULD NOT');
endmon;
*inlr = *on;
*inlr = *on;
// Write one line to gn1.out in the job's current directory (DSPLY under
// `system` goes to QSYSOPR, a queue PUB400 users share).
dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
