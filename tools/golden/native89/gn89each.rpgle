**FREE
// Stage 2, S2.89: FOR-EACH over whole arrays. A packed array summed, a
// character array into a shorter field (cut), a LEAVE inside the loop,
// and two loops nested. Captured on PUB400 (tools/golden/native89.txt
// n89_foreach) for the native unit NFOREACH2.
ctl-opt dftactgrp(*no);
dcl-s nums packed(5:2) dim(4);
dcl-s names char(6) dim(3);
dcl-s n packed(7:2);
dcl-s tot packed(9:2);
dcl-s s3 char(3);
dcl-s a char(6);
dcl-s line varchar(52);
nums(1) = 1.25;
nums(2) = 10;
nums(3) = -3.5;
nums(4) = 0.75;
names(1) = 'alpha';
names(2) = 'beta';
names(3) = 'gamma';
tot = 0;
for-each n in nums;
  tot += n;
endfor;
line = 'SUM ' + %char(tot) + ' LAST ' + %char(n);
for-each s3 in names;
  line += ' ' + s3;
endfor;
say(line);
line = 'LEAVE';
for-each n in nums;
  if n < 0;
    leave;
  endif;
  line += ' ' + %char(n);
endfor;
line += ' NEST';
for-each a in names;
  for-each n in nums;
    if n > 1;
      line += ' ' + %trim(a) + %char(n);
    endif;
  endfor;
endfor;
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
