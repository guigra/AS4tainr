**FREE
// Stage 2, S2.30: subfields of a data structure array, ds(i).field, with
// literal and field indexes, in loops, conditions and assignments, and out
// of range. Captured on PUB400 (tools/golden/native38.txt n38_dsa) for the
// native unit test NDSA.
ctl-opt dftactgrp(*no);
dcl-ds item qualified dim(3);
  code char(2);
  qty packed(5:0);
end-ds;
dcl-s i int(10);
dcl-s j int(10) inz(3);
dcl-s k int(10) inz(4);
dcl-s tot packed(7:0);
for i = 1 to 3;
  item(i).code = 'C' + %char(i);
  item(i).qty = i * 10;
endfor;
say('DSA LIT [' + item(2).code + '] ' + %char(item(2).qty));
for i = 1 to 3;
  tot = tot + item(i).qty;
endfor;
say('DSA SUM ' + %char(tot));
if item(j).qty > 20 and item(j).code = 'C3';
  say('DSA COND YES');
else;
  say('DSA COND NO');
endif;
item(1).code = item(j).code;
say('DSA COPY [' + item(1).code + ']');
monitor;
  tot = item(k).qty;
  say('DSA PAST ' + %char(tot));
on-error;
  say('DSA PAST ERR ' + %char(%status));
endmon;
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
