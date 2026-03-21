import paper from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedpaper = addPrefix(paper, prefix);
  addBase({ ...prefixedpaper });
};
