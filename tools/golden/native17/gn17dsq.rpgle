**FREE
// Stage 2, S2.16: qualified subfields (QUALIFIED and LIKEDS). Captured on
// PUB400 (tools/golden/native17.txt n17_dsq) for the native unit test NDSQ.
ctl-opt dftactgrp(*no);
dcl-ds cust qualified;
  id packed(5:0);
  name char(10);
end-ds;
dcl-ds tmpl qualified template;
  a char(3);
  n zoned(3:0);
end-ds;
dcl-ds rec likeds(tmpl);
cust.id = 42;
cust.name = 'ACME';
cust.id = cust.id + 1;
say('DSQ CUST ' + %char(cust.id) + ' [' + cust.name + ']');
rec.n = cust.id * 2;
rec.a = %trim(cust.name);
say('DSQ LIKEDS ' + %char(rec.n) + ' [' + rec.a + ']');
if rec.n > 80 and %subst(cust.name:1:2) = 'AC';
  say('DSQ COND YES');
else;
  say('DSQ COND NO');
endif;
say('DSQ BYTES [' + %subst(rec:1:3) + ']');
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
