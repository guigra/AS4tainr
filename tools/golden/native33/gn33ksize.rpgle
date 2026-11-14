**FREE
// %SIZE of numeric named constants, for an upstream fix.
ctl-opt dftactgrp(*no);
dcl-c K1 12345;
dcl-c K2 12.5;
dcl-c K3 1234567;
dcl-c K4 -5;
dcl-c K5 0;
dcl-c K6 123456789012;
dcl-c K7 .25;
dcl-c K8 100.00;
dcl-s n int(10);
n = %size(K1);
say('SIZE 12345 ' + %char(n));
n = %size(K2);
say('SIZE 12.5 ' + %char(n));
n = %size(K3);
say('SIZE 1234567 ' + %char(n));
n = %size(K4);
say('SIZE -5 ' + %char(n));
n = %size(K5);
say('SIZE 0 ' + %char(n));
n = %size(K6);
say('SIZE 123456789012 ' + %char(n));
n = %size(K7);
say('SIZE .25 ' + %char(n));
n = %size(K8);
say('SIZE 100.00 ' + %char(n));
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
