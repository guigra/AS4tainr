**FREE
// Two reference parameters that are the same variable.
ctl-opt dftactgrp(*no);
dcl-s v int(10) inz(0);
both(v : v);
say('ALIAS V=' + %char(v));
*inlr = *on;
dcl-proc both;
  dcl-pi *n;
    p1 int(10);
    p2 int(10);
  end-pi;
  p1 = 1;
  p2 = 2;
  say('ALIAS P1=' + %char(p1) + '|P2=' + %char(p2));
end-proc;
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
