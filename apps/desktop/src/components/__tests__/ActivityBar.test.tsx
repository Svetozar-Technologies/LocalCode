import React from 'react';
import { describe, it, expect, beforeEach } from 'vitest';
import { render, fireEvent } from '@testing-library/react';
import ActivityBar from '../ActivityBar/ActivityBar';
import { useAppStore } from '../../stores/appStore';

beforeEach(() => {
  useAppStore.setState({
    sidebarView: 'explorer',
  });
});

describe('ActivityBar', () => {
  it('renders all sidebar view icons', () => {
    render(<ActivityBar />);
    const items = document.querySelectorAll('.activity-bar-item');
    // explorer, search, git, ai, debug, settings, composer = 7
    expect(items.length).toBe(7);
  });

  it('marks the active view', () => {
    useAppStore.setState({ sidebarView: 'search' });
    render(<ActivityBar />);
    const items = document.querySelectorAll('.activity-bar-item');
    const searchItem = Array.from(items).find((el) => el.getAttribute('title') === 'Search');
    expect(searchItem?.classList.contains('active')).toBe(true);
  });

  it('changes sidebar view on click', () => {
    render(<ActivityBar />);
    const items = document.querySelectorAll('.activity-bar-item');
    const gitItem = Array.from(items).find((el) => el.getAttribute('title') === 'Source Control');
    if (gitItem) fireEvent.click(gitItem);
    expect(useAppStore.getState().sidebarView).toBe('git');
  });

  it('has Composer icon', () => {
    render(<ActivityBar />);
    const items = document.querySelectorAll('.activity-bar-item');
    const composerItem = Array.from(items).find((el) => el.getAttribute('title') === 'Composer');
    expect(composerItem).toBeDefined();
  });
});
