**FREE
// FOR-EACH over an array (the example of IBM Docs "FOR-EACH (For Each)").
dcl-s order_states varchar(10) dim(3);
dcl-s state varchar(10);
order_states(1) = 'Open';
order_states(2) = 'Active';
order_states(3) = 'Closed';
for-each state in order_states;
  dsply state;
endfor;
*inlr = *on;
