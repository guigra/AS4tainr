**FREE
// Native unit NNEST (S2.76): a path with one index on its middle part
// (list.users(i).name), a literal and a field index, one past the
// elements caught by MONITOR. The program captured on PUB400, with DSPLY
// for its output.
// Source: golden capture tools/golden/native76 n76_nested (PUB400, IBM i
// 7.5, 2026-11-18).
dcl-ds user_t qualified template;
  name char(10);
  days zoned(3:0);
end-ds;
dcl-ds list qualified;
  count int(10);
  users likeds(user_t) dim(3);
end-ds;
dcl-s i int(10);
dcl-s line varchar(52);
list.count = 3;
list.users(1).name = 'ADA';
list.users(1).days = 5;
i = 2;
list.users(i).name = 'GRACE';
list.users(i).days = list.users(1).days + 7;
list.users(3).name = 'LIN';
list.users(3).days = list.users(i).days * 2;
line = 'U';
for i = 1 to list.count;
  line += ' ' + %trim(list.users(i).name) + '=' + %char(list.users(i).days);
endfor;
dsply (line);
i = 4;
monitor;
  list.users(i).days = 1;
  dsply ('NO ERROR');
on-error;
  dsply ('ERROR ' + %char(%status));
endmon;
*inlr = *on;
